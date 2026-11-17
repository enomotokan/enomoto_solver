"""HiGHS の前処理の規則を 1 つずつ止めて、減る列の数が変わるかを見る: python highs_ruleoff.py 問題 [規則番号,...]
(presolve_rule_off のビット。止められるのは 5 番以降)。結果は標準出力"""
import sys, highspy
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
p = sys.argv[1]
rules = [int(x) for x in sys.argv[2].split(",")] if len(sys.argv) > 2 else list(range(5, 24))
def run(off):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 4)
    if off: h.setOptionValue("presolve_rule_off", off)
    h.readModel(str(WT / ".miplib_cache" / f"{p}.mps")); h.presolve()
    lp = h.getPresolvedLp(); return lp.num_row_, lp.num_col_, len(lp.a_matrix_.value_)
print(p, "全部", run(0), flush=True)
for r in rules:
    print(p, "規則", r, "を止める", run(1 << r), flush=True)
