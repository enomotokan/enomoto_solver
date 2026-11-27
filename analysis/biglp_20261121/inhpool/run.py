"""引き継ぎの鎖を共有の待ち行列でも共有する直し (`ENOMOTO_MIP_INHPOOL`、既定で使う) の前後を比べる (analysis/biglp_20261121/README.md の 8 節)。
使い方: python run.py OUT.jsonl [問題:スレッド:秒,...]  (省くと JOBS を全部)。各仕事を old (`ENOMOTO_MIP_INHPOOL=0`) と new (既定) で 1 回ずつ。
途中から再開できる。0.5 秒ごとに子プロセスの RSS を読み、最大を残す。stderr のうち `memc`・`TC r`・`tree search` の行を logs/ に残す。種 1。"""
import json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = "neos5 misc07 noswot mik-250-20-75-4 neos-911970".split()
JOBS = [f"{p}:1:60" for p in P] + [f"{p}:4:60" for p in P] + ["n3div36:4:600", "rail01:4:600", "supportcase18:4:300", "unitcal_7:4:300"]
OUT = Path(sys.argv[1])
jobs = sys.argv[2].split(",") if len(sys.argv) > 2 else JOBS
done = {(r["job"], r["mode"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
(D / "logs").mkdir(exist_ok=True)


def rss(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"):
                return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0


for job in jobs:
    p, th, tl = job.split(":")
    for mode in ("old", "new"):
        if (job, mode) in done:
            continue
        mps = rb.fetch(p, WT / ".miplib_cache")
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED="1")
        if mode == "old":
            env["ENOMOTO_MIP_INHPOOL"] = "0"
        t0 = time.time()
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", th],
                              cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        keep = []

        def pump():
            for l in pr.stderr:
                if l.startswith(("MIP: memc", "MIP: TC r", "MIP: tree search")):
                    keep.append(l)
        th_ = threading.Thread(target=pump, daemon=True)
        th_.start()
        peak = 0.0
        while pr.poll() is None:
            # 子 (ワーカー) の RSS: run_miplib_benchmark の --worker-ours は同じプロセスで解く
            peak = max(peak, rss(pr.pid))
            time.sleep(0.5)
        out = pr.stdout.read()
        th_.join(5)
        try:
            res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[-200:]}
        (D / "logs" / f"{p}_t{th}_{mode}.log").write_text("".join(keep))
        r = {"job": job, "mode": mode, "problem": p, "threads": int(th), "tl": int(tl), "result": res, "max_rss_mb": round(peak), "wall": round(time.time() - t0, 1)}
        with OUT.open("a") as f:
            f.write(json.dumps(r) + "\n")
        print(job, mode, res.get("status"), res.get("objective"), res.get("bound"), res.get("nodes"), round(peak), flush=True)
