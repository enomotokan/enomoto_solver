"""P2-INF: ROWTINY の正しさの確かめ (analysis/wrong_widden_20261117/dbgrun.py の写しにスレッド数を足した形)。
python wdn.py 問題 TAG 種の範囲 正しい最適値 スレッド [KEY=VAL ...]。最小化の目的値が最適値 + 1e-6 相対を超える optimal を誤答とする。時間上限は環境変数 TL (既定 900)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
prob, tag, rng, opt, th = sys.argv[1], sys.argv[2], sys.argv[3], float(sys.argv[4]), sys.argv[5]
extra = dict(a.split("=", 1) for a in sys.argv[6:])
a, b = map(int, rng.split("-"))
mps = Path(prob) if prob.endswith(".mps") else rb.fetch(prob, WT / ".miplib_cache")
tl = os.environ.get("TL", "900")
(D / "logs").mkdir(exist_ok=True)
for s in range(a, b + 1):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": th}); env.update(extra); env.update({"ENOMOTO_T_MIP_SEED": str(s), "ENOMOTO_MIP_LOG": "1"})
    t0 = time.time()
    p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", th], cwd=WT, env=env, capture_output=True, text=True)
    try: res = json.loads([l for l in p.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "rc": p.returncode}  # noqa: BLE001
    o = res.get("objective")
    wrong = res.get("status") == "optimal" and o is not None and o > opt + 1e-6 * (1 + abs(opt))
    ls = p.stderr.splitlines()
    (D / "logs" / f"{tag}_s{s}.log").write_text("\n".join([l for l in ls if "MIP_DEBUG" in l or "LPINF" in l][:3000] + ["---- 最後の 40 行 ----"] + ls[-40:]) + "\n")
    with open(D / f"res_dbgrun_{tag}.jsonl", "a") as f:
        f.write(json.dumps({"problem": prob, "tag": tag, "seed": s, "th": th, "env": extra, "status": res.get("status"), "obj": o, "nodes": res.get("nodes"), "time": res.get("time"), "wrong": wrong}) + "\n")
    print(tag, s, res.get("status"), o, res.get("nodes"), round(time.time() - t0, 1), "WRONG" if wrong else "", flush=True)
