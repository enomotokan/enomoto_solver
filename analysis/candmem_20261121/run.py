"""候補のメモリと密なカットの比べ (2026-11-21、親の依頼、analysis/biglp_20261121 の直す案 2)。
使い方: python run.py <old の展開先> <new の展開先> [仕事...]  (仕事は 問題:設定:種、res.jsonl に足す。同じ仕事は飛ばす = 続きから)。
設定 (CFGS): old・new はビルド (PYTHONPATH で切り替える)、ほかは new のビルドに環境変数を足す (密なカットの既存の手、どれも既定では使わない)。
4 スレッド・ENOMOTO_MIP_LOG=1・600 秒 (環境変数 TL で変える)。stderr の各行に経過秒を付けて logs/<問題>_<設定>_s<種>.log、
0.5 秒ごとの RSS (MB) とスレッド数を logs/<問題>_<設定>_s<種>.rss。ソルバーは同時に 1 つ。"""
import json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
CFGS = {
    "old": ("old", {}),
    "new": ("new", {}),
    # HiGHS のように点数を非零の数で割る (`ENOMOTO_T_CUTSEL_DENSITY_EXP`)
    "dx1": ("new", {"ENOMOTO_T_CUTSEL_DENSITY_EXP": "1"}),
    # HiGHS の postprocessCut と同じく、カットの非零を 100 + 0.15 × 列 までにする (`ENOMOTO_MIP_GAP4` の `l`、既定は rocn)
    "gl": ("new", {"ENOMOTO_MIP_GAP4": "rocnl"}),
    # 根の LP が重い問題で 1 ラウンドに足すカットの非零の和を抑える (`ENOMOTO_MIP_ROOTCAP` の `d`)
    "rcd": ("new", {"ENOMOTO_MIP_ROOTCAP": "d"}),
}
TL = float(os.environ.get("TL", "600"))
OUT = D / "res.jsonl"
def stat(pid):
    rss = th = 0
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): rss = int(l.split()[1]) / 1024
            elif l.startswith("Threads:"): th = int(l.split()[1])
    except OSError:
        pass
    return rss, th
def run(job, builds):
    p, c, seed = job.split(":")
    b, extra = CFGS[c]
    mps = WT / ".miplib_cache" / f"{p}.mps"
    base = D / "logs" / f"{p}_{c}_s{seed}"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=seed, PYTHONPATH=builds[b], **extra)
    cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"]
    t0 = time.time()
    pr = subprocess.Popen(cmd, cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    def errpump():
        with open(f"{base}.log", "w") as f:
            for l in pr.stderr:
                f.write(f"[{time.time() - t0:8.2f}] {l}")
    th_ = threading.Thread(target=errpump, daemon=True); th_.start()
    peak = 0.0; peak_t = 0.0
    with open(f"{base}.rss", "w") as fr:
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
if __name__ == "__main__":
    builds = {"old": sys.argv[1], "new": sys.argv[2]}
    done = {json.loads(l)["job"] for l in OUT.open()} if OUT.exists() else set()
    for job in sys.argv[3:]:
        if job in done: continue
        res, peak, peak_t, wall = run(job, builds)
        with open(OUT, "a") as f:
            f.write(json.dumps({"job": job, "tl": TL, "result": res, "max_rss_mb": peak, "peak_at": peak_t, "wall": wall, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(job, res.get("status"), res.get("time"), peak, flush=True)
