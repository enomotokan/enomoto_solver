"""HiGHS の効いたカットが ENOMOTO の候補にあるか: python cand_match.py <cutlp_r(k-1).mps> <cands_rk.txt> <chosen_rk.txt> <pool json>
分離の点 x = cutlp_r(k-1) の LP 解。HiGHS のカット (双対非零のもの) ごとに、x での違反・効き目と、ENOMOTO の候補の中で一番平行なもの (余弦) を出す"""
import json, sys
import numpy as np, highspy
from collections import Counter
mps, cf, chf, pj = sys.argv[1:5]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps); h.run()
x = np.array(h.getSolution().col_value); n = len(x)
print(f"x from {mps}: LP {h.getInfo().objective_function_value:.4f}")
def load(f):
    out = []
    for line in open(f):
        t = line.split(); src, eff, rhs = int(t[0]), float(t[1]), float(t[2])
        d = {}
        for kv in t[3:]:
            j, a = kv.split(":"); d[int(j)] = float(a)
        out.append((src, eff, rhs, d))
    return out
C = load(cf); CH = load(chf)
names = ["extra", "rows", "zerohalf", "clique", "implbd", "path_highs", "path", "tableau", "pool"]
print("ENOMOTO candidates", len(C), Counter(names[s % 16] if s % 16 < 9 else str(s) for s, *_ in C).most_common(), "chosen", len(CH), Counter(names[s % 16] if s % 16 < 9 else str(s) for s, *_ in CH).most_common())
print("  top efficacies cands:", [round(e, 3) for e in sorted((c[1] for c in C), reverse=True)[:10]])
P = json.load(open(pj)); S, I, V = P["start"], P["index"], P["value"]
act = json.load(open(pj + ".active.json"))["active"]
def cos(a, b):
    num = sum(v * b.get(j, 0.0) for j, v in a.items())
    return num / (np.sqrt(sum(v * v for v in a.values())) * np.sqrt(sum(v * v for v in b.values())))
for c in act:
    idx = I[S[c]:S[c + 1]]; val = V[S[c]:S[c + 1]]
    d = dict(zip(idx, val))
    lb, ub = P["lower"][c], P["upper"][c]
    if ub >= 1e30:  # >= の形は符号を返す
        d = {j: -v for j, v in d.items()}; ub = -lb
    a = sum(v * x[j] for j, v in d.items()); viol = a - ub
    eff = viol / np.sqrt(sum(v * v for v in d.values()))
    if viol <= 1e-6:
        continue
    best = max(((cos(d, cd), names[s % 16] if s % 16 < 9 else str(s), e) for s, e, r, cd in C if set(cd) & set(d)), default=(0, "-", 0))
    bestch = max(((cos(d, cd), names[s % 16] if s % 16 < 9 else str(s)) for s, e, r, cd in CH if set(cd) & set(d)), default=(0, "-"))
    print(f"  HiGHS cut {c} nnz {len(d)} viol {viol:.3f} eff {eff:.3f} | best cand cos {best[0]:.3f} ({best[1]}, eff {best[2]:.3f}) | best chosen cos {bestch[0]:.3f} ({bestch[1]})")
