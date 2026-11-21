"""大きな問題の根までの時間とメモリの診断 (2026-11-21、親の依頼)。既定は変えない。
使い方: python run.py [仕事...]  (仕事は 問題:ソルバー、省くと JOBS を全部)。res.jsonl に足す (同じ仕事は飛ばす = 続きから)。
ソルバー: ours4 / ours1 (当方の既定、4 / 1 スレッド、ENOMOTO_MIP_LOG=1、600 秒、種 1)、scip / highs (one_other.py、根で止める nodes=1、600 秒)。
- 当方は stderr の各行に開始からの経過秒を付けて logs/<問題>_<ソルバー>.log に書く (段ごとの時刻を出すため)。
- どのソルバーも 0.5 秒ごとに子プロセスの RSS (VmRSS、MB) と スレッド数を logs/<問題>_<ソルバー>.rss に書く (経過秒 RSS スレッド数)。
- ソルバーは同時に 1 つ (4 コアの機械)。"""
import json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
PROBS = ["cod105", "sing44", "rail01", "n3div36", "sorrell3"]
JOBS = [f"{p}:{s}" for s in ("ours4", "ours1", "scip", "highs") for p in PROBS]
TL = 600
OUT = D / "res.jsonl"
done = {json.loads(l)["job"] for l in OUT.open()} if OUT.exists() else set()
def stat(pid):
    rss = th = 0
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): rss = int(l.split()[1]) / 1024
            elif l.startswith("Threads:"): th = int(l.split()[1])
    except OSError:
        pass
    return rss, th
def run(job):
    p, s = job.split(":")
    mps = WT / ".miplib_cache" / f"{p}.mps"
    log = D / "logs" / f"{p}_{s}.log"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    if s.startswith("ours"):
        th = s[4:]
        env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED="1")
        cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", th]
    else:
        cmd = [sys.executable, str(D / "one_other.py"), s, str(mps), str(TL), str(log), str(D / "logs" / f"{p}_{s}.stat"), "1"]
    t0 = time.time()
    pr = subprocess.Popen(cmd, cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    def errpump():
        with open(log, "w") if s.startswith("ours") else open(os.devnull, "w") as f:
            for l in pr.stderr:
                f.write(f"[{time.time() - t0:8.2f}] {l}")
    th_ = threading.Thread(target=errpump, daemon=True); th_.start()
    peak = 0.0; peak_t = 0.0
    with open(D / "logs" / f"{p}_{s}.rss", "w") as fr:
        while pr.poll() is None:
            rss, nth = stat(pr.pid); t = time.time() - t0
            fr.write(f"{t:.1f} {rss:.0f} {nth}\n")
            if rss > peak: peak, peak_t = rss, t
            time.sleep(0.5)
            if t > TL + 300: pr.kill(); break
    out = pr.stdout.read(); th_.join(5)
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    return res, round(peak), round(peak_t, 1), round(time.time() - t0, 1)
for job in (sys.argv[1:] or JOBS):
    if job in done: continue
    res, peak, peak_t, wall = run(job)
    with open(OUT, "a") as f:
        f.write(json.dumps({"job": job, "result": res, "max_rss_mb": peak, "peak_at": peak_t, "wall": wall, "at": time.strftime("%H:%M:%S")}) + "\n")
    print(job, res.get("status"), res.get("time"), peak, flush=True)
