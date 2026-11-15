"""mc11_wait1: 根の LP の競争 (二段解法と内点法 + クロスオーバー) の勝ち負けと道の分かれを数える。
主の根の LP の反復・勝った側 (ENOMOTO_DEBUG_RACE)・主の根のカットのループの終わりの値・写しの 1 ラウンド目を、短い時間の上限で集める。
使い方: python race.py 問題 設定,... 回数 [時間の上限 (既定 12)]   ログ logs_race/<問題>_<設定>_<回>.log、結果 race.jsonl (設定は run.py の CFGS)"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from run import CFGS  # noqa: E402
P, cfgs, n = sys.argv[1], sys.argv[2].split(","), int(sys.argv[3])
tl = sys.argv[4] if len(sys.argv) > 4 else "12"
TAG = os.environ.get("W1_TAG", "")
(D / "logs_race").mkdir(exist_ok=True)
mps = rb.fetch(P, WT / ".miplib_cache")
done = {}
if (D / "race.jsonl").exists():
    for l in open(D / "race.jsonl"):
        r = json.loads(l); done[(r["problem"], r["cfg"])] = max(done.get((r["problem"], r["cfg"]), -1), r["run"])
for i in range(n):
    for c in cfgs:
        k = done.get((P, c + TAG), -1) + 1
        env = {k2: v for k2, v in os.environ.items() if not k2.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", ENOMOTO_DEBUG_RACE="1", **CFGS[c])
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=300)
        t = r.stderr + r.stdout
        (D / "logs_race" / f"{P}_{c}{TAG}_{k}.log").write_text(t)
        d = {"problem": P, "cfg": c + TAG, "run": k}
        m = re.search(r"^MIP: root LP (\S+) \((\d+) iters", t, re.M); d["root_iters"] = int(m.group(2)) if m else None
        m = re.search(r"MIP root race: (\w+) won", t); d["race"] = m.group(1) if m else None
        m = re.search(r"cut loop done: .*-> (\S+)", t); d["main_cut"] = float(m.group(1)) if m else None
        m = re.search(r"rounds so far \[([\d.]+) at", t); d["bg_r1"] = float(m.group(1)) if m else None
        m = re.search(r"background cuts: (\d+) rounds, \d+ cut rows \(was \d+\), obj \S+ -> (\S+)", t); d["bg_obj"] = float(m.group(2)) if m else None
        js = [l for l in r.stdout.splitlines() if l.startswith("{")]
        if js:
            res = json.loads(js[-1]); d.update(status=res["status"], time=res["time"], root_bound=res.get("root_bound"))
        with open(D / "race.jsonl", "a") as f:
            f.write(json.dumps(d) + "\n")
        done[(P, c + TAG)] = k
        print(json.dumps(d), flush=True)
