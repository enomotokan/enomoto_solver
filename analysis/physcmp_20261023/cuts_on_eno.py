"""HiGHS のカットプール (前処理後の列番号) を元の列名で ENOMOTO の前処理後の問題 (dump、列は元の順の C<j>) に足して LP を解く。
python cuts_on_eno.py <HiGHS 前処理後 mps> <pool json> <ENOMOTO dump mps>"""
import json, sys
import numpy as np, highspy
hp, pj, ep = sys.argv[1:4]
P = json.load(open(pj))
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(hp)
hn = list(h.getLp().col_names_)
e = highspy.Highs(); e.setOptionValue("output_flag", False); e.readModel(ep)
elp = e.getLp()
en = {nm: j for j, nm in enumerate(elp.col_names_)}
lo, up = elp.col_lower_, elp.col_upper_
def emap(name):  # HiGHS 'cK' -> ENOMOTO 'C(K-1)'
    return en["C%d" % (int(name[1:]) - 1)]
e.setOptionValue("solve_relaxation", True); e.setOptionValue("presolve", "off"); e.run()
base = e.getInfo().objective_function_value
act = set(json.load(open(pj + ".active.json"))["active"])
# 5 番目の引数: ENOMOTO の同値な組 (ENOMOTO_MIP_DEBUG_DUMP_PROBE の .pairs: j k neg) を等式として足す
if len(sys.argv) > 4:
    m0 = 0
    for line in open(sys.argv[4]):
        j, k, neg = map(int, line.split())
        if neg:
            e.addRow(1, 1, 2, np.array([j, k], dtype=np.int32), np.array([1.0, 1.0]))
        else:
            e.addRow(0, 0, 2, np.array([j, k], dtype=np.int32), np.array([1.0, -1.0]))
        m0 += 1
    e.run()
    print(f"ENOMOTO dump LP {base:.4f} + {m0} equivalences: {e.getInfo().objective_function_value:.4f}")
st, ix, vl = P["start"], P["index"], P["value"]
added = skipped = act_added = act_skipped = 0
fixed_terms = 0
for c in range(len(st) - 1):
    idx = [emap(hn[j]) for j in ix[st[c]:st[c + 1]]]; val = vl[st[c]:st[c + 1]]
    lb, ub = P["lower"][c], P["upper"][c]
    # ENOMOTO で固定された列は値を右辺に移す。代入で消えた列 (固定と区別できない) も固定扱いになる点に注意
    free_i, free_v, shift = [], [], 0.0
    for j, v in zip(idx, val):
        if lo[j] == up[j]:
            shift += v * lo[j]; fixed_terms += 1
        else:
            free_i.append(j); free_v.append(v)
    if not free_i:
        skipped += 1; act_skipped += c in act; continue
    e.addRow(lb - shift if lb > -1e30 else lb, ub - shift if ub < 1e30 else ub, len(free_i), np.array(free_i, dtype=np.int32), np.array(free_v))
    added += 1; act_added += c in act
e.run()
print(f"ENOMOTO dump LP {base:.4f} -> + HiGHS cuts {added} (skipped {skipped}, active added {act_added}/{len(act)}, fixed terms {fixed_terms}): {e.getInfo().objective_function_value:.4f} status {e.modelStatusToString(e.getModelStatus())}")
