"""探りなしの書き出し B に、探りありの A の違いを 1 つずつ入れた MPS を作る (MPS の BOUNDS を書き換える):
python -I premod.py <A.mps> <B.mps> <出力の接頭辞>  -> <接頭辞>_fix180.mps (180 本の固定)、_zlb.mps (z の下限 7607.99999209)、_zlb8.mps (z の下限 7608)"""
import sys
import highspy, numpy as np
a, b, pre = sys.argv[1:4]
def load(f):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f); return h
ha, hb = load(a), load(b)
la, lb = ha.getLp(), hb.getLp()
na = {ha.getColName(j)[1]: j for j in range(la.num_col_)}
nb = {hb.getColName(j)[1]: j for j in range(lb.num_col_)}
fix = [k for k, j in nb.items() if lb.col_lower_[j] < lb.col_upper_[j] and k in na and la.col_lower_[na[k]] == la.col_upper_[na[k]]]
z = "C62277"
def write(name, cl_changes):
    h = load(b)
    for k, (lo, up) in cl_changes.items():
        j = nb[k]; h.changeColBounds(j, lo, up)
    h.writeModel(f"{pre}_{name}.mps")
    print(name, len(cl_changes))
write("fix180", {k: (la.col_lower_[na[k]], la.col_upper_[na[k]]) for k in fix})
write("zlb", {z: (la.col_lower_[na[z]], la.col_upper_[na[z]])})
write("zlb8", {z: (7608.0, la.col_upper_[na[z]])})
