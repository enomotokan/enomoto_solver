"""再起動に強い比較用の実行器。

使い方: paired.py SPEC.json
SPEC: {"out": "結果.jsonl", "threads": 4, "time": 60, "rounds": 2,
       "configs": [{"name": "base", "env": {}}, {"name": "x", "env": {"ENOMOTO_...": "1"}}], "problems": [...] (省略で 40 問)}

- 問題ごとに全ての設定を続けて解く (設定の順は問題ごとにずらす)。1 つの (回, 問題) の組を「グループ」と呼ぶ。
- 結果は 1 行ずつ追記して fsync する。各行にマシンの boot_id と CPU を記録する。
- 起動すると、終わった実行を飛ばして続きから始める。boot_id が今と違う、終わっていないグループの結果は捨てて解き直す
  (グループの中は同じマシンで測ったものだけになる。比較はグループの中の対で行う)。
- 何度起動しても同じ (既に動いていれば何もしない: ロックファイル)。
"""
import fcntl
import json
import os
import subprocess
import sys

sys.path.insert(0, "/home/user/enomoto_solver/scripts")
import run_miplib_benchmark as rb

spec = json.load(open(sys.argv[1]))
out = spec["out"]
lock = open(out + ".lock", "w")
try:
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
except BlockingIOError:
    print("already running", flush=True)
    sys.exit(0)
boot = open("/proc/sys/kernel/random/boot_id").read().strip()
cpu = next((l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name")), "?")
ncpu = os.cpu_count()
threads, tl, rounds = spec["threads"], float(spec["time"]), spec["rounds"]
cfgs = spec["configs"]
names = spec.get("problems") or rb.PROBLEMS
py = spec.get("python", sys.executable)

recs = [json.loads(l) for l in open(out)] if os.path.exists(out) else []
groups = {}
for r in recs:
    groups.setdefault((r["round"], r["name"]), []).append(r)
keep = []
for key, rs in groups.items():
    complete = {r["cfg"] for r in rs} >= {c["name"] for c in cfgs}
    boots = {r["boot"] for r in rs}
    if complete and len(boots) == 1:
        keep += rs
    else:
        keep += [r for r in rs if r["boot"] == boot]  # 途中のグループ: 今のマシンの分だけ残す
if len(keep) != len(recs):
    tmp = out + ".tmp"
    with open(tmp, "w") as f:
        for r in keep:
            f.write(json.dumps(r) + "\n")
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, out)
    print(f"dropped {len(recs) - len(keep)} results from unfinished groups of an earlier boot", flush=True)
done = {(r["round"], r["name"], r["cfg"]) for r in keep}

for rnd in range(1, rounds + 1):
    for i, n in enumerate(names):
        k = (i + rnd) % len(cfgs)
        for c in cfgs[k:] + cfgs[:k]:
            if (rnd, n, c["name"]) in done:
                continue
            env = dict(os.environ, ENOMOTO_PROF_TREE_PAR="1", ENOMOTO_MIP_THREADS=str(threads), **c["env"])
            cmd = [py, rb.__file__, "--worker-ours", f"/home/user/enomoto_solver/.miplib_cache/{n}.mps",
                   "--time-limit", str(tl), "--threads", str(threads)]
            try:
                p = subprocess.run(cmd, capture_output=True, text=True, timeout=tl * 1.5 + 60, env=env, cwd="/tmp")
                res = next((json.loads(l) for l in reversed(p.stdout.splitlines()) if l.startswith("{")),
                           {"status": f"crash {p.returncode}"})
                prof = [l[9:] for l in p.stderr.splitlines() if l.startswith("TREEPAR: ")]
            except subprocess.TimeoutExpired:
                res, prof = {"status": "killed"}, []
            rec = {"name": n, "cfg": c["name"], "round": rnd, "boot": boot, "cpu": cpu, "ncpu": ncpu, "threads": threads, **res, "prof": prof}
            with open(out, "a") as f:
                f.write(json.dumps(rec) + "\n")
                f.flush()
                os.fsync(f.fileno())
            print(rnd, n, c["name"], res.get("status"), res.get("time"), flush=True)
print("all done", flush=True)
