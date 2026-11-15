"""問題の組 B を長い時間の上限で 1 回ずつ解き、最大メモリ (RSS) の推移・ログの大きさを取る (2026-11-03)。
使い方: python analysis/mittelmann_ready_20261103/long_b/run_long.py {ours|highs} 時間の上限[秒] 問題 ...
- worker は scripts/run_miplib_benchmark.py の --worker-ours / --worker-highs (4 スレッド、ENOMOTO_MIP_THREADS=4)。
- ours は ENOMOTO_MIP_LOG=1 で標準エラーをログファイル (作業用の場所、LOGDIR) に書く。
- 5 秒ごとに /proc から RSS (子孫を含む和) とログの大きさを取り、結果は long_<kind>.jsonl に 1 行ずつ足す。
  最大 RSS は getrusage(RUSAGE_CHILDREN) の ru_maxrss (/usr/bin/time -v の「Maximum resident set size」と同じ値)。"""
import json, os, resource, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
LOGDIR = Path(os.environ.get("LOGDIR", "/tmp/claude-0/longb_logs")); LOGDIR.mkdir(parents=True, exist_ok=True)
kind, tl, probs = sys.argv[1], float(sys.argv[2]), sys.argv[3:]
TAG = os.environ.get("LONGB_TAG", "")  # 環境変数を変えた回の名前 (ファイル名と記録に付ける)

def rss_tree(pid):
    """pid とその子孫の RSS の和 [MB]"""
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
    log = LOGDIR / f"{kind}{TAG}_{p}_{int(tl)}.log"
    env = dict(os.environ, ENOMOTO_MIP_THREADS="4")
    if kind in ("ours", "ourslog"): env["ENOMOTO_MIP_LOG"] = "1"
    cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(tl), "--threads", "4"]
    if kind == "ourslog":  # 同じ求解 + log=True (全スレッドの開いたノードの数の経過を標準出力に)
        cmd = [sys.executable, str(D / "diag_worker.py"), str(mps), str(tl)]
    t0 = time.time(); series = []
    with open(log, "w") as ferr, open(LOGDIR / f"{kind}{TAG}_{p}_{int(tl)}.out", "w") as fout:
        pr = subprocess.Popen(cmd, cwd=WT, stdout=fout, stderr=ferr, env=env)
        killed = False; ru = None
        while True:
            wp, st, ru = os.wait4(pr.pid, os.WNOHANG)
            if wp: break
            time.sleep(5)
            el = time.time() - t0
            series.append([round(el, 1), round(rss_tree(pr.pid), 1), log.stat().st_size])
            if el > tl * 1.5 + 120:
                pr.kill(); killed = True
        wall = time.time() - t0
    out = (LOGDIR / f"{kind}{TAG}_{p}_{int(tl)}.out").read_text().splitlines()
    js = [l for l in out if l.startswith("{")]
    res = json.loads(js[-1]) if js else {"status": "crash" if not killed else "killed"}
    rc = os.waitstatus_to_exitcode(st)
    rec = {"problem": p, "kind": kind, "tag": TAG, "env": {k: v for k, v in os.environ.items() if k.startswith("ENOMOTO_")}, "time_limit": tl, "result": res, "wall": round(wall, 1), "rc": rc,
           "maxrss_mb": round(ru.ru_maxrss / 1024, 1), "cpu_s": round(ru.ru_utime + ru.ru_stime, 1),  # wait4 でこの子だけの値
           "peak_rss_mb_sampled": max((s[1] for s in series), default=None), "log_bytes": log.stat().st_size,
           "warnings": sum(1 for l in open(log, errors="replace") if "MIP: WARNING" in l), "series": series,
           "at": time.strftime("%H:%M:%S")}
    with open(D / f"long_{kind}{TAG}.jsonl", "a") as f: f.write(json.dumps(rec) + "\n")
    print(p, kind, res.get("status"), res.get("objective"), res.get("bound"), rec["peak_rss_mb_sampled"], rec["log_bytes"], flush=True)
