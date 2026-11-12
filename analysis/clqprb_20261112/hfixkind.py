"""CLQPRB: HiGHS だけが 0 に固定した列の標本で、x_j = 1 を課した元の問題が (a) LP 緩和で実行不能か、(b) 整数で実行不能か
(目的 0、前処理なし、10 秒) を調べる。両方とも実行可能なら双対の議論 (最適解の集合を狭める) による固定。
使い方: python hfixkind.py 問題 標本の数"""
import sys, random, json
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent; WT = D.parents[1]
p, ns = sys.argv[1], int(sys.argv[2])
k = {}
for l in open(D / "h" / f"{p}_hfix.tsv"):
    j, t, v = l.split(); k[int(j)] = (t, float(v))
ho = [int(l) for l in open(D / "h" / f"{p}_honly.txt") if l.strip()]
cand = [j for j in ho if k[j] == ("fix", 0.0)]
random.seed(1); smp = random.sample(cand, min(ns, len(cand)))
res = []
for j in smp:
    out = {"col": j}
    for mode in ("lp", "mip", "mip_pre"):
        h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1)
        h.readModel(str(WT / ".miplib_cache" / f"{p}.mps"))
        lp = h.getLp()
        h.changeColBounds(j, 1.0, 1.0)
        if mode == "lp":
            h.changeColsIntegrality(lp.num_col_, list(range(lp.num_col_)), [highspy.HighsVarType.kContinuous] * lp.num_col_)
        else:
            h.changeColsCost(lp.num_col_, list(range(lp.num_col_)), [0.0] * lp.num_col_)
            h.setOptionValue("time_limit", 20.0)
            h.setOptionValue("mip_max_improving_sols", 1) if False else None
            h.setOptionValue("presolve", "on" if mode == "mip_pre" else "off")
        h.run()
        out[mode] = str(h.getModelStatus()).split(".")[-1]
        if mode == "lp" and out[mode] == "kInfeasible": break
        if mode == "mip" and out[mode] in ("kOptimal", "kInfeasible"): break
    print(json.dumps(out), flush=True); res.append(out)
