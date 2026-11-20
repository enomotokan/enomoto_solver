"""OBJTARGET 1: 暫定解の推移 (`ENOMOTO_MIP_LOG=1` の `MIP: new incumbent Z ... (Ts)` の行) を取る。
python traj.py 名前 秒 種 問題... → res_traj_名前.jsonl に足す (問題ごとに暫定解の (時刻, 値) の列)。環境変数 (ENOMOTO_*) はそのまま渡す。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
name, TL, seed, probs = sys.argv[1], float(sys.argv[2]), sys.argv[3], sys.argv[4:]
OUT = D / f"res_traj_{name}.jsonl"
done = {(json.loads(l)["problem"], json.loads(l)["seed"]) for l in OUT.read_text().splitlines()} if OUT.exists() else set()
pat = re.compile(r"MIP: new incumbent ([-+0-9.eE]+) .*?\(([0-9.]+)s\)")
for p in probs:
    if (p, int(seed)) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = dict(os.environ); env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": seed})
    err = tempfile.TemporaryFile("w+"); t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                        cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True, timeout=TL + 300)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    err.seek(0)
    lines = err.read().splitlines()
    inc = [(float(m.group(2)), float(m.group(1))) for l in lines if (m := pat.search(l))]
    tp = [l for l in lines if "TPRE" in l or "Presolve:" in l][:20]
    row = {"problem": p, "seed": int(seed), "tl": TL, "env": {k: v for k, v in os.environ.items() if k.startswith("ENOMOTO_")}, "result": res, "inc": inc, "tpre": tp, "wall": round(time.time() - t0, 1)}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(p, res.get("status"), res.get("objective"), len(inc), inc[-1] if inc else None, flush=True)
