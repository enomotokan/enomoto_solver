"""CLQPRB: HiGHS の前処理で残った列と ENOMOTO の前処理後 (ENOMOTO_MIP_DEBUG_PRESOLVE_COLS) の列を突き合わせる。
使い方: python cmpcols.py 問題 enomoto_cols.tsv  → h/<問題>_cmp.json と、HiGHS だけが消した列の番号 h/<問題>_honly.txt"""
import json, sys
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent; WT = D.parents[1]
p, et = sys.argv[1], sys.argv[2]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1)
h.readModel(str(WT / ".miplib_cache" / f"{p}.mps"))
names = list(h.getLp().col_names_)
h.presolve(); keep = set(h.getPresolvedLp().col_names_)
efix = {}
for line in open(et):
    j, lo, up, nz = line.split(); j = int(j)
    if float(lo) == float(up) or int(nz) == 0: efix[j] = float(lo)
hrem = {j for j, n in enumerate(names) if n not in keep}
both = hrem & set(efix); honly = hrem - set(efix); eonly = set(efix) - hrem
r = {"n": len(names), "highs_removed": len(hrem), "enomoto_removed": len(efix), "both": len(both), "highs_only": len(honly), "enomoto_only": len(eonly)}
print(json.dumps(r))
(D / "h").mkdir(exist_ok=True)
(D / "h" / f"{p}_cmp.json").write_text(json.dumps(r) + "\n")
(D / "h" / f"{p}_honly.txt").write_text("\n".join(str(j) for j in sorted(honly)) + "\n")
(D / "h" / f"{p}_eonly.txt").write_text("\n".join(f"{j}\t{efix[j]}" for j in sorted(eonly)) + "\n")
