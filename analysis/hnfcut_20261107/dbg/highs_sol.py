"""前処理の後の問題 (列の名前 C{j}) を HiGHS で解き、列の番号の順に値を 1 行ずつ書く (ENOMOTO_MIP_DEBUG_SOL 用)。
使い方: python highs_sol.py in.mps out.txt [時間]"""
import sys, re, highspy
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0)
h.setOptionValue("time_limit", float(sys.argv[3]) if len(sys.argv) > 3 else 600.0)
h.readModel(sys.argv[1]); h.run()
print(h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value)
lp = h.getLp(); x = h.getSolution().col_value
names = [h.getColName(i)[1] for i in range(lp.num_col_)]
n = max(int(re.sub(r"\D", "", s)) for s in names) + 1
v = [0.0] * n
for s, val in zip(names, x):
    v[int(s[1:])] = round(val) if abs(val - round(val)) < 1e-6 else val
open(sys.argv[2], "w").write("\n".join(repr(float(a)) for a in v) + "\n")
