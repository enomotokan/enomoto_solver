"""cmz (`ENOMOTO_MIP_CUTMIX=z`) が zero-half を入れる問題の見当: 元の MPS で、全部整数の列・整数の係数で係数の絶対値が 2 以下の行の割合
(cutmix.rs の int_small。本物は前処理の後の問題で数えるので目安)。python intsmall.py > intsmall.txt"""
import json, sys
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(D.parents[1] / "cbcimpl_20261029")); sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS
import run_miplib_benchmark as rb
M = sorted({t[0] for s in json.load(open(D / "shards_M.json"))["shards"] for t in s})
def frac(p):
    mps = rb.fetch(p, WT / ".miplib_cache")
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps))
    lp = h.getLp(); m = lp.num_row_
    integ = list(lp.integrality_) if len(lp.integrality_) else [highspy.HighsVarType.kContinuous] * lp.num_col_
    isint = [t != highspy.HighsVarType.kContinuous for t in integ]
    a = lp.a_matrix_; st, idx, val = list(a.start_), list(a.index_), list(a.value_)
    ok = [True] * m; amax = [0.0] * m; cnt = [0] * m
    for j in range(lp.num_col_):
        for k in range(st[j], st[j + 1]):
            i = idx[k]; v = val[k]; cnt[i] += 1
            if not isint[j] or abs(v - round(v)) > 1e-9: ok[i] = False
            amax[i] = max(amax[i], abs(v))
    n = sum(1 for i in range(m) if cnt[i] > 0 and ok[i] and amax[i] <= 2 + 1e-9)
    return n / max(1, m)
for name, lst in (("P", PROBS), ("M", M)):
    for p in lst:
        try: f = frac(p)
        except Exception as e: print(name, p, "error", str(e)[:60], flush=True); continue
        print(name, p, f"{f:.3f}", "z" if f >= 0.5 else "", flush=True)
