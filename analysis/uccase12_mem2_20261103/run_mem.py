"""uccase12 などを 1 回解き、RSS の推移と全スレッドの済んだノード・開いたノードを取る (2026-11-03、long_b/run_long.py の ourslog を元に)。
使い方: MEM_TAG=名前 [環境変数...] python analysis/uccase12_mem2_20261103/run_mem.py 時間の上限[秒] 問題 ...
- 子は long_b/diag_worker.py (4 スレッド、log=True で経過を標準出力に)。ENOMOTO_MIP_LOG=1 の標準エラーは LOGDIR に書く。
- 5 秒ごとに /proc から RSS (子孫を含む和) を取る。RSS が MEM_CAP_MB (既定 12500) を超えたら子を止めて「cap」とする
  (この機械は 15 GB で、cgroup の OOM killer に落とされる前に止める)。最大 RSS は wait4 の ru_maxrss。
- 結果は runs.jsonl に 1 行ずつ足す。経過行 (`<秒>s <済んだ> <開いた> <下界> <暫定解> <ギャップ>`) は prog に間引いて入れる。"""
import json, os, re, subprocess, sys, time, signal
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
LOGDIR = Path(os.environ.get("LOGDIR", "/tmp/claude-0/mem2_logs")); LOGDIR.mkdir(parents=True, exist_ok=True)
tl, probs = float(sys.argv[1]), sys.argv[2:]
TAG = os.environ.get("MEM_TAG", "def")
CAP = float(os.environ.get("MEM_CAP_MB", "12500"))
PROG = re.compile(r"^\s+([0-9.]+)s\s+(\d+)\s+(\d+)\s+(\S+)\s+(\S+)\s+(\S+)")

def rss_tree(pid):
    tot, stack = 0, [pid]
    while stack:
        q = stack.pop()
        try:
            for l in open(f"/proc/{q}/status"):
                if l.startswith("VmRSS:"): tot += int(l.split()[1])
            for t in os.listdir(f"/proc/{q}/task"):
                stack += [int(c) for c in open(f"/proc/{q}/task/{t}/children").read().split()]
        except (FileNotFoundError, ProcessLookupError):
            pass
    return tot / 1024

for p in probs:
    mps = WT / ".miplib_cache" / f"{p}.mps"
    base = LOGDIR / f"{TAG}_{p}_{int(tl)}"
    env = dict(os.environ, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1")
    cmd = [sys.executable, str(WT / "analysis/mittelmann_ready_20261103/long_b/diag_worker.py"), str(mps), str(tl)]
    t0 = time.time(); series = []; capped = False
    with open(f"{base}.log", "w") as ferr, open(f"{base}.out", "w") as fout:
        pr = subprocess.Popen(cmd, cwd=WT, stdout=fout, stderr=ferr, env=env)
        while True:
            wp, st, ru = os.wait4(pr.pid, os.WNOHANG)
            if wp: break
            time.sleep(5)
            el = time.time() - t0
            r = rss_tree(pr.pid)
            series.append([round(el, 1), round(r, 1)])
            if (r > CAP or el > tl * 1.5 + 120) and not capped:
                capped = True; pr.send_signal(signal.SIGKILL)
        wall = time.time() - t0
    out = Path(f"{base}.out").read_text().splitlines()
    js = [l for l in out if l.startswith("{")]
    res = json.loads(js[-1]) if js else {"status": "cap" if capped else "crash"}
    prog, last = [], -1e9
    for m in map(PROG.match, out):
        if m and float(m.group(1)) - last >= 10:
            last = float(m.group(1)); prog.append([float(m.group(1)), int(m.group(2)), int(m.group(3)), m.group(4), m.group(5), m.group(6)])
    rec = {"problem": p, "tag": TAG, "env": {k: v for k, v in os.environ.items() if k.startswith(("ENOMOTO_", "MALLOC"))}, "time_limit": tl,
           "result": res, "wall": round(wall, 1), "rc": os.waitstatus_to_exitcode(st), "capped": capped,
           "maxrss_mb": round(ru.ru_maxrss / 1024, 1), "cpu_s": round(ru.ru_utime + ru.ru_stime, 1),
           "series": series, "prog": prog, "at": time.strftime("%H:%M:%S")}
    with open(D / "runs.jsonl", "a") as f: f.write(json.dumps(rec) + "\n")
    print(p, TAG, res.get("status"), res.get("objective"), res.get("bound"), "maxrss", rec["maxrss_mb"], "capped" if capped else "", flush=True)
