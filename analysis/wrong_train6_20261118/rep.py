"""traininstance6 の誤答の頻度 (wrong_widden_20261117/rep.py の写し): python rep.py TAG 種の範囲 回数 [THREADS] [KEY=VAL ...]
例: python rep.py def 1-8 2 4。res_TAG.jsonl に 1 行ずつ足す。正しい最適は 28290 (誤答は 28290.5 より大きい optimal)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
tag, rng, reps = sys.argv[1], sys.argv[2], int(sys.argv[3])
th = sys.argv[4] if len(sys.argv) > 4 else "4"
extra = dict(a.split("=", 1) for a in sys.argv[5:])
a, b = map(int, rng.split("-"))
mps = rb.fetch("traininstance6", WT / ".miplib_cache")
tl = float(os.environ.get("TL", "600"))
for r in range(reps):
    for s in range(a, b + 1):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(extra)
        if "ENOMOTO_MIP_DEBUG_SOL" in extra or "ENOMOTO_MIP_DEBUG_LPINF" in extra: env["ENOMOTO_MIP_LOG"] = "1"
        env["ENOMOTO_MIP_THREADS"] = th; env["ENOMOTO_T_MIP_SEED"] = str(s)
        t0 = time.time()
        p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", th],
                           cwd=WT, env=env, capture_output=True, text=True)
        try: res = json.loads([l for l in p.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "rc": p.returncode, "err": p.stderr[-300:]}  # noqa: BLE001
        if "ENOMOTO_MIP_DEBUG_SOL" in extra or "ENOMOTO_MIP_DEBUG_LPINF" in extra:  # デバッグ解つき: MIP_DEBUG_SOL の行 (最初の 300 行) と最後の 40 行を logs/ に残す
            ls = p.stderr.splitlines(); (D / "logs").mkdir(exist_ok=True)
            (D / "logs" / f"{tag}_s{s}_r{r}.log").write_text("\n".join([l for l in ls if "MIP_DEBUG_SOL" in l or "LPINF" in l or "new incumbent" in l][:5000] + ["---- 最後の 40 行 ----"] + ls[-40:]) + "\n")
        o = res.get("objective")
        wrong = res.get("status") == "optimal" and o is not None and o > 28290.5
        row = {"tag": tag, "seed": s, "rep": r, "th": th, "env": extra, "status": res.get("status"), "obj": o, "bound": res.get("bound"),
               "nodes": res.get("nodes"), "time": res.get("time"), "wrong": wrong}
        with open(D / f"res_{tag}.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(tag, s, r, res.get("status"), o, res.get("nodes"), round(time.time() - t0, 1), "WRONG" if wrong else "", flush=True)
