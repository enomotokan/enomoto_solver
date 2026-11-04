"""1 問を 1 回解き、RSS と smaps の分類 (mimalloc の領域・glibc の主ヒープ・glibc のスレッドの領域) の推移を取る (2026-11-04、memcap_b)。
使い方: MEM_TAG=<名前> [環境変数...] python run_mem.py <時間の上限[秒]> <問題> [heaptrack]
- 求解は long_b/diag_worker.py (log=True、4 スレッド)。5 秒ごとに /proc/<pid>/smaps を読む (uccase12_mem1 の watch_proc.py と同じ分類)。
- 結果は runs.jsonl に 1 行足し、ログ (stderr・stdout) は LOGDIR (既定 /tmp/memb_logs) に置く。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
LOGDIR = Path(os.environ.get("LOGDIR", "/tmp/memb_logs")); LOGDIR.mkdir(parents=True, exist_ok=True)
tl, prob = float(sys.argv[1]), sys.argv[2]
ht = len(sys.argv) > 3 and sys.argv[3] == "heaptrack"
tag = os.environ.get("MEM_TAG", "def")
name = f"{tag}_{prob}_{int(tl)}"
mps = WT / ".miplib_cache" / f"{prob}.mps"
env = dict(os.environ, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1")
cmd = [sys.executable, str(WT / "analysis/mittelmann_ready_20261103/long_b/diag_worker.py"), str(mps), str(tl)]
if ht:
    cmd = ["heaptrack", "-o", str(LOGDIR / f"ht_{name}")] + cmd


def find_py(pid):
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
    """RSS [MB] と smaps の分類 [MB]"""
    cat = {"mi_1g": 0, "heap": 0, "arena64": 0, "arena64_n": 0, "anon_other": 0, "file": 0}
    cur = None
    try:
        for l in open(f"/proc/{pid}/smaps"):
            if re.match(r"^[0-9a-f]+-[0-9a-f]+ ", l):
                f = l.split()
                a, b = (int(x, 16) for x in f[0].split("-"))
                cur = [(b - a) >> 20, f[5] if len(f) > 5 else ""]
            elif l.startswith("Rss:") and cur is not None:
                rs = int(l.split()[1]) >> 10
                v, nm = cur
                if nm == "[heap]": cat["heap"] += rs
                elif nm: cat["file"] += rs
                elif v >= 1000: cat["mi_1g"] += rs
                elif 60 <= v <= 64: cat["arena64"] += rs; cat["arena64_n"] += 1
                else: cat["anon_other"] += rs
        rss = [int(l.split()[1]) >> 10 for l in open(f"/proc/{pid}/status") if l.startswith("VmRSS")][0]
    except (OSError, IndexError):
        return None
    return rss, cat


log = LOGDIR / f"{name}.log"; out = LOGDIR / f"{name}.out"
t0 = time.time(); series = []; maxrss = 0
with open(log, "w") as fe, open(out, "w") as fo:
    pr = subprocess.Popen(cmd, cwd=WT, stdout=fo, stderr=fe, env=env)
    py = None
    while pr.poll() is None:
        time.sleep(5)
        py = py if py and os.path.exists(f"/proc/{py}") else find_py(pr.pid)
        if py:
            s = sample(py)
            if s:
                series.append([round(time.time() - t0, 1), s[0], s[1]])
                maxrss = max(maxrss, s[0])
        if time.time() - t0 > tl * 2 + 300:
            pr.kill()
wall = time.time() - t0
lines = out.read_text().splitlines()
js = [l for l in lines if l.startswith("{")]
res = json.loads(js[-1]) if js else {"status": "crash", "rc": pr.returncode}
PROG = re.compile(r"^\s+([0-9.]+)s\s+(\d+)\s+(\d+)\s+(\S+)\s+(\S+)\s+(\S+)")
prog = [[float(g[0]), int(g[1]), int(g[2]), g[3], g[4]] for g in (m.groups() for m in map(PROG.match, lines) if m)]
diag = [l for l in log.read_text(errors="replace").splitlines() if l.startswith("MEMB_DIAG") or "MEM2:" in l]
rec = {"name": name, "tag": tag, "problem": prob, "time_limit": tl, "heaptrack": ht,
       "env": {k: v for k, v in os.environ.items() if k.startswith(("ENOMOTO_", "MALLOC_", "MIMALLOC_"))},
       "result": res, "rc": pr.returncode, "wall": round(wall, 1), "peak_rss_mb_sampled": maxrss, "series": series,
       "progress": prog[::max(1, len(prog) // 60)], "diag": diag, "at": time.strftime("%Y-%m-%d %H:%M:%S")}
with open(D / "runs.jsonl", "a") as f:
    f.write(json.dumps(rec) + "\n")
print(name, res.get("status"), res.get("objective"), res.get("bound"), res.get("nodes"), maxrss, round(wall, 1), flush=True)
