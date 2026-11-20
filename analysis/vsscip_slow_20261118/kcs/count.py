"""KEEP_COSTED_SINGLETON が働く問題の見当: 元の MPS で、費用のついた連続列で、等式 1 本だけに出る列 (列シングルトン等式) の数。
本物は前処理の途中の問題で決まるので目安。python count.py > count.txt  (行: 組 問題 列 該当列)"""
import json, sys
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(D.parents[1] / "cbcimpl_20261029")); sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS
import run_miplib_benchmark as rb
M = sorted({t[0] for s in json.load(open(D.parent / "domrev" / "shards_M.json"))["shards"] for t in s} | {"neos-5107597-kakapo", "neos-5195221-niemur"})
def count(p):
    mps = rb.fetch(p, WT / ".miplib_cache")
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps))
    lp = h.getLp(); n = lp.num_col_
    integ = list(lp.integrality_) if len(lp.integrality_) else [highspy.HighsVarType.kContinuous] * n
    c = list(lp.col_cost_); rl = list(lp.row_lower_); ru = list(lp.row_upper_)
    a = lp.a_matrix_; st = list(a.start_); idx = list(a.index_)
    k = 0
    for j in range(n):
        if integ[j] != highspy.HighsVarType.kContinuous or c[j] == 0.0 or st[j + 1] - st[j] != 1: continue
        i = idx[st[j]]
        if rl[i] == ru[i]: k += 1
    return n, k
for name, lst in (("P", PROBS), ("M", M)):
    for p in lst:
        try: n, k = count(p)
        except Exception as e: print(name, p, "error", str(e)[:60], flush=True); continue
        print(name, p, n, k, flush=True)
