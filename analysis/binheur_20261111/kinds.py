"""58 問と res8 の 47 問の型を調べる: 0-1 変数だけ (連続・一般整数なし) かどうかと、行の形 (集合分割・集合被覆・集合パッキング・
個数 (係数がすべて 1 で右辺が 2 以上)・その他) の本数。kinds.json に書く。使い方: python kinds.py"""
import json, sys
from collections import Counter
from pathlib import Path
import numpy as np
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
RES8 = json.loads((WT / "analysis/mitt600_20261103/shards8.json").read_text())["problems"]
out = {}
for p in list(dict.fromkeys(PROBS + RES8)):
    mps = rb.fetch(p, WT / ".miplib_cache")
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps))
    lp = h.getLp(); n = lp.num_col_; m = lp.num_row_
    integ = np.array([int(v) for v in lp.integrality_]) if len(lp.integrality_) else np.zeros(n, int)
    lo = np.array(lp.col_lower_); up = np.array(lp.col_upper_)
    isbin = (integ == 1) & (lo >= 0) & (up <= 1)
    nb = int(isbin.sum()); ngi = int(((integ == 1) & ~isbin).sum()); nc = int((integ != 1).sum())
    a = lp.a_matrix_
    st = np.array(a.start_); idx = np.array(a.index_); val = np.array(a.value_)
    # 行ごとに: 全部 0-1 列か、係数が全部 +1 (または全部 -1) か
    rows_allbin = np.ones(m, bool); rows_pm1 = np.ones(m, bool); rows_sign = np.zeros(m)
    rnz = np.zeros(m, int)
    for j in range(n):
        r = idx[st[j]:st[j + 1]]; v = val[st[j]:st[j + 1]]
        rnz[r] += 1
        if not isbin[j]:
            rows_allbin[r] = False
        rows_pm1[r[np.abs(np.abs(v) - 1) > 1e-12]] = False
        rows_sign[r] += np.sign(v)
    rl = np.array(lp.row_lower_); ru = np.array(lp.row_upper_)
    kind = Counter()
    for i in range(m):
        if rnz[i] == 0:
            continue
        if not (rows_allbin[i] and rows_pm1[i] and abs(rows_sign[i]) == rnz[i]):
            kind["other"] += 1; continue
        s = 1 if rows_sign[i] > 0 else -1
        lo_, up_ = (rl[i], ru[i]) if s > 0 else (-ru[i], -rl[i])
        if lo_ == up_:
            kind["partition" if lo_ == 1 else "card_eq"] += 1
        elif lo_ >= 1 and up_ > 1e20:
            kind["cover" if lo_ == 1 else "card_ge"] += 1
        elif up_ < 1e20 and lo_ < -1e20 + 1e30:
            kind["packing" if up_ == 1 else "card_le"] += 1
        else:
            kind["other"] += 1
    out[p] = {"n": n, "m": m, "bin": nb, "genint": ngi, "cont": nc, "pure01": ngi == 0 and nc == 0,
              "in58": p in PROBS, "inres8": p in RES8, "rows": dict(kind)}
    print(p, out[p], flush=True)
(D / "kinds.json").write_text(json.dumps(out, indent=0))
