"""combb の 600 秒の比べ (2026-11-06)。analysis/mitt600_20261103/run_cfg.py の写しで、ENOMOTO_MIP_LOG=1 のログを logs/<問題>_<設定>_r<回>_<秒>.log.gz に残し、
回の番号は名前だけ (種は変えない。前の計測と同じく回ごとの差はスレッドの揺れ。RUN_SEED を与えたときだけ ENOMOTO_T_MIP_SEED に渡す)。設定は analysis/halve_submip_20261030/run.py の CFGS。
使い方: RUN_CFG_TL=600 python analysis/combb_600_20261106/run.py 設定 問題 回  → res_<秒>.jsonl に 1 行足す"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis/halve_submip_20261030"))
import run_miplib_benchmark as rb  # noqa: E402
from run import CFGS  # noqa: E402
# この計測だけの切り分けの設定 (combb の一部を外す・条件を変える)
CB = CFGS["combb"]
CFGS = dict(CFGS)
CFGS["combb_m17k"] = dict(CB, ENOMOTO_T_SIMPLEX_SCALE_MIN_M="17000")  # 尺度の調整を 17000 行以上だけ (supportcase42 の根 18439 行は調整、RINS の部分 MIP 16616 行は調整なし)
CFGS["combb_noscale"] = {k: v for k, v in CB.items() if "SCALE" not in k}  # HARRIS2=r と NOISE_RESET だけ
CFGS["combb_scaleonly"] = {k: v for k, v in CB.items() if "SCALE" in k}  # 尺度の調整だけ
TL = int(os.environ.get("RUN_CFG_TL", "600"))
cfg, p, rep = sys.argv[1], sys.argv[2], int(sys.argv[3])
OUT = D / (f"res_{TL}.jsonl" if os.environ.get("RUN_THREADS", "4") == "4" else f"res_{TL}_t{os.environ['RUN_THREADS']}.jsonl")
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
TH = os.environ.get("RUN_THREADS", "4")  # 切り分け用 (1 スレッドで同じ道筋か見る)。4 以外ならログ・結果の名前に _t<数> が付く
env.update(ENOMOTO_MIP_THREADS=TH, ENOMOTO_MIP_LOG="1", **CFGS[cfg])
if os.environ.get("RUN_SEED"):
    env["ENOMOTO_T_MIP_SEED"] = os.environ["RUN_SEED"]
logf = D / "logs" / (f"{p}_{cfg}_r{rep}_{TL}.log" if TH == "4" else f"{p}_{cfg}_r{rep}_{TL}_t{TH}.log")
t0 = time.time(); peak = 0.0
with open(logf, "w") as ef:
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", TH],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=ef, text=True)
    while pr.poll() is None:
        try:
            rss = next(int(l.split()[1]) / 1024 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS:"))
        except (OSError, StopIteration):
            rss = 0.0
        peak = max(peak, rss)
        time.sleep(1)
        if time.time() - t0 > TL + 300: pr.kill(); break
    out, _ = pr.communicate()
try:
    res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode, "error": open(logf, errors="replace").read()[-300:]}
with open(logf, "rb") as a, gzip.open(str(logf) + ".gz", "wb") as b:
    b.write(a.read())
logf.unlink()
with open(OUT, "a") as f:
    f.write(json.dumps({"problem": p, "solver": "ours", "cfg": cfg, "rep": rep, "threads": int(TH), "result": res, "max_rss_mb": round(peak), "wall": round(time.time() - t0, 1),
                        "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
print(p, cfg, rep, res.get("status"), res.get("time"), round(peak), flush=True)
