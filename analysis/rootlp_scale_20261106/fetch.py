"""候補の問題を .miplib_cache に取り、元の問題の行数・係数の幅を meta_orig.jsonl に書く。使い方: python fetch.py"""
import json, sys, gzip
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
from run_miplib_benchmark import fetch
c = json.load(open(D / "candidates.json"))
names = c["bench_mitt"] + c["coll_sample"]
done = {json.loads(l)["problem"] for l in open(D / "meta_orig.jsonl")} if (D / "meta_orig.jsonl").exists() else set()
def coef(p):
    import highspy, numpy as np
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    h.readModel(str(WT / ".miplib_cache" / f"{p}.mps"))
    lp = h.getLp(); v = np.abs(np.array(lp.a_matrix_.value_)); v = v[v > 0]
    return dict(problem=p, rows=lp.num_row_, cols=lp.num_col_, nz=len(v), range=float(v.max() / v.min()) if len(v) else 0)
def one(p):
    try:
        fetch(p, WT / ".miplib_cache"); return p, None
    except Exception as e: return p, str(e)
with ThreadPoolExecutor(6) as ex:
    for p, e in ex.map(one, [p for p in names if p not in done]):
        if e: print("fail", p, e, flush=True); continue
        r = coef(p)
        with open(D / "meta_orig.jsonl", "a") as f: f.write(json.dumps(r) + "\n")
        print(r, flush=True)
