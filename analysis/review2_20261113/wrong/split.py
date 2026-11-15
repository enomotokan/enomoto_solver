"""unitcal_7 の誤答 (実行可能なのに infeasible) の切り分け: python split.py [種] [設定...]。
設定: def・base・cand1113 (RCSTRONG=irk)・base から 1 項目を外した形 (-KEY)、それぞれに +KEY=VAL で足す (例 base+ENOMOTO_MIP_UNB_FEAS=0)。結果は split.jsonl に足す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D.parent))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import BASE  # noqa: E402
P = os.environ.get("PROB", "unitcal_7")
mps = rb.fetch(P, WT / ".miplib_cache")
seed = int(sys.argv[1])
names = sys.argv[2:] or (["def", "base"] + [f"-{k}" for k in BASE])
for n in names:
    head, *extra = n.split("+")
    if head == "def": env = {}
    elif head == "base": env = dict(BASE)
    elif head == "cand1113": env = {**BASE, "ENOMOTO_MIP_RCSTRONG": "irk"}
    else: env = None
    if env is not None:
        # 例: base+ENOMOTO_MIP_UNB_FEAS=0、def+ENOMOTO_MIP_CLQPRB=1d
        for kv in extra:
            k, v = kv.split("="); env[k] = v
    elif n.startswith("-"): env = {k: v for k, v in BASE.items() if k != n[1:]}
    else: raise SystemExit(n)
    e = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_") or k == "ENOMOTO_MIP_LOG"}
    e.update(env); e["ENOMOTO_MIP_THREADS"] = "4"; e["ENOMOTO_T_MIP_SEED"] = str(seed)
    t0 = time.time()
    out = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", os.environ.get("TL", "40"), "--threads", "4"],
                         cwd=WT, env=e, capture_output=True, text=True)
    out, err = out.stdout, out.stderr
    if os.environ.get("ENOMOTO_MIP_LOG"): (D / f"log_{P}_{n.replace('/', '_')[:80]}_{seed}.txt").write_text(err + out)
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    row = {"problem": P, "cfg": n, "seed": seed, "status": res.get("status"), "time": res.get("time"), "nodes": res.get("nodes"), "obj": res.get("objective"), "wall": round(time.time() - t0, 1)}
    with open(D / "split.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
    print(row, flush=True)
