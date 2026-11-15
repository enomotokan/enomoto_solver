"""puriri_cutlp (2026-11-06): 問題を ENOMOTO の設定ごとに 4 スレッドで解き、ログを logs/ に、結果を res.jsonl に足す。
使い方: python run.py CFG TL REP PROB [PROB ...]
CFG は cfgs の鍵 (環境変数の組。def は何も足さない)。ログは ENOMOTO_MIP_LOG=1 の標準エラー (analysis/highs_only_20261105/G4/run.py と同じ形)。"""
import json, os, subprocess, sys, time, shutil
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
sys.path.insert(0, str(D))
from cfgs import CFGS  # noqa: E402
cfg, tl, rep = sys.argv[1], int(sys.argv[2]), sys.argv[3]
for p in sys.argv[4:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env["ENOMOTO_MIP_THREADS"] = "4"
    env["ENOMOTO_MIP_LOG"] = "1"
    env.update(CFGS[cfg])
    final = D / "logs" / f"{p}_{cfg}_{tl}_r{rep}.log"
    log = Path("/tmp") / ("cls_" + final.name)
    t0 = time.time()
    with open(log, "w") as lf:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps),
                             "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True)
    shutil.move(str(log), str(final))
    try:
        res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode}
    # 根の終わりの時刻 (カットのループの終わりの行の時刻) を拾う
    with open(D / "res.jsonl", "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "tl": tl, "rep": rep, "env": CFGS[cfg], "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S"), "result": res}) + "\n")
    print(p, cfg, rep, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), res.get("nodes"), flush=True)
