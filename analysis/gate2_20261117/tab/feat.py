"""根だけ (node_limit 1、60 秒、4 スレッド、種 1) を既定 + ENOMOTO_MIP_DEBUG_SEP=1 で解き、主の根の tableau の分離の行
("SEP tableau ...") と前処理後の大きさをログから拾って feat.jsonl に足す。python feat.py <札> [K=V ...]
(対象は slow2_20261117/shards_P.json の 58 問)"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
tag = sys.argv[1]; kv = [a for a in sys.argv[2:] if "=" in a]
P = sorted({x[0] for sh in json.loads((WT / "analysis/slow2_20261117/shards_P.json").read_text())["shards"] for x in sh})
OCODE = """
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
mo, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = mo.solve(None, False, 60.0, None, 1, 4, True); t = time.perf_counter() - t
print("RES " + json.dumps({k: r.get(k) for k in ("status", "objective", "best_bound", "nodes", "lp_iterations", "root_bound")} | {"time": t}))
"""
if os.environ.get("FEAT_PROBS"): P = os.environ["FEAT_PROBS"].split(",")  # 一部の問題だけ
OUT = D / "feat.jsonl"
done = {(json.loads(l)["problem"], json.loads(l)["tag"]) for l in OUT.read_text().splitlines()} if OUT.exists() else set()
(D / "logs").mkdir(exist_ok=True)
for p in P:
    if (p, tag) in done: continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_DEBUG_SEP": "1"})
    for x in kv:
        k, v = x.split("=", 1); env[k] = v
    t0 = time.time()
    try:
        pr = subprocess.run([sys.executable, "-c", OCODE, str(WT / ".miplib_cache" / f"{p}.mps")], env=env, cwd=WT, capture_output=True, text=True, timeout=240)
        out = pr.stdout + pr.stderr
    except subprocess.TimeoutExpired as e:
        out = (e.stdout or b"").decode(errors="replace") + (e.stderr or b"").decode(errors="replace")
    ls = out.splitlines()
    tab = [l for l in ls if (l.startswith("SEP tableau") and "CBC" not in l) or "tableau rule" in l]
    (D / "logs" / f"feat_{p}_{tag}.log.gz").write_bytes(gzip.compress("\n".join([l for l in ls if not l.startswith("SEP ")][:400] + tab[:200]).encode()))
    res = [l for l in ls if l.startswith("RES ")]
    r = json.loads(res[-1][4:]) if res else {"status": "crash"}
    row = {"problem": p, "tag": tag, "env": kv, "wall": round(time.time() - t0, 1), "tab": tab[:40]} | r
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(p, r.get("status"), r.get("root_bound"), len(tab), flush=True)
