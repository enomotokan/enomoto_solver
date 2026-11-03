"""uccase12 などを 1 回解き、RSS・smaps_rollup の推移と全スレッドの開いたノードの数を取る (2026-11-03、uccase12_mem1)。
使い方: python run_mem.py <名前> <問題> <時間の上限[秒]> [heaptrack]
- 求解は long_b/diag_worker.py (log=True、4 スレッド)。標準出力の経過行 (全スレッドの済んだノード・開いたノード) を残す。
- 2 秒ごとに /proc/<pid>/smaps_rollup の Rss・Anonymous と /proc/<pid>/status の Threads を取る。
- 4 番目の引数に heaptrack を与えると `heaptrack -o` の下で動かす (デバッグ情報つきの wheel は PYTHONPATH で与える)。
- 結果は mem_<名前>.jsonl に 1 行足し、ログ (stderr・stdout) は LOGDIR (既定 /tmp/claude-0/mem1_logs) に置く。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
LOGDIR = Path(os.environ.get("LOGDIR", "/tmp/claude-0/mem1_logs")); LOGDIR.mkdir(parents=True, exist_ok=True)
name, prob, tl = sys.argv[1], sys.argv[2], float(sys.argv[3])
ht = len(sys.argv) > 4 and sys.argv[4] == "heaptrack"
mps = WT / ".miplib_cache" / f"{prob}.mps"
env = dict(os.environ, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1")
cmd = [sys.executable, str(WT / "analysis/mittelmann_ready_20261103/long_b/diag_worker.py"), str(mps), str(tl)]
if ht:
    cmd = ["heaptrack", "-o", str(LOGDIR / f"ht_{name}")] + cmd


def find_py(pid):
    """heaptrack の下では python は子 (孫) になる。python の pid を探す"""
    stack = [pid]
    while stack:
        q = stack.pop()
        try:
            if "python" in open(f"/proc/{q}/comm").read():
                return q
            for t in os.listdir(f"/proc/{q}/task"):
                stack += [int(c) for c in open(f"/proc/{q}/task/{t}/children").read().split()]
        except OSError:
            pass
    return None


def sample(pid):
    r = {}
    try:
        for l in open(f"/proc/{pid}/smaps_rollup"):
            k = l.split(":")[0]
            if k in ("Rss", "Anonymous"):
                r[k] = int(l.split()[1]) // 1024
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("Threads:"):
                r["th"] = int(l.split()[1])
    except OSError:
        pass
    return r


log = LOGDIR / f"{name}.log"; out = LOGDIR / f"{name}.out"
t0 = time.time(); series = []; maxrss = 0
with open(log, "w") as fe, open(out, "w") as fo:
    pr = subprocess.Popen(cmd, cwd=WT, stdout=fo, stderr=fe, env=env)
    py = None
    while pr.poll() is None:
        time.sleep(2)
        py = py if py and os.path.exists(f"/proc/{py}") else find_py(pr.pid)
        if py:
            s = sample(py)
            if s:
                series.append([round(time.time() - t0, 1), s.get("Rss", 0), s.get("Anonymous", 0), s.get("th", 0)])
                maxrss = max(maxrss, s.get("Rss", 0))
        if time.time() - t0 > tl * 2 + 300:
            pr.kill()
wall = time.time() - t0
lines = out.read_text().splitlines()
js = [l for l in lines if l.startswith("{")]
res = json.loads(js[-1]) if js else {"status": "crash", "rc": pr.returncode}
PROG = re.compile(r"^\s+([0-9.]+)s\s+(\d+)\s+(\d+)\s+(\S+)\s+(\S+)\s+(\S+)")
prog = [[float(g[0]), int(g[1]), int(g[2]), g[3], g[4]] for g in (m.groups() for m in map(PROG.match, lines) if m)]
rec = {"name": name, "problem": prob, "time_limit": tl, "heaptrack": ht,
       "env": {k: v for k, v in os.environ.items() if k.startswith(("ENOMOTO_", "MALLOC_", "MIMALLOC_"))},
       "result": res, "rc": pr.returncode, "wall": round(wall, 1), "peak_rss_mb_sampled": maxrss, "series": series, "progress": prog,
       "at": time.strftime("%Y-%m-%d %H:%M:%S")}
with open(D / f"mem_{name}.jsonl", "a") as f:
    f.write(json.dumps(rec) + "\n")
print(name, res.get("status"), res.get("objective"), res.get("bound"), maxrss, round(wall, 1), flush=True)
