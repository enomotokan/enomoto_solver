"""計装した HiGHS (highs_tag.patch) の HIGHS_POOL_TAG の出力を集計する。
使い方: python -I tagsumm.py 模型.mps pooltag.txt [詳しく見る本数]
- P 行: カットプールに入ったカット (submip, プール番号, 行番号, 出どころ, 方法, 衝突か, 右辺, 長さ | 列 係数 ... | 集約した行)
  出どころ: tab / path / modk (HighsSeparation の分離器)、impl (含意)、clq (クリーク)、cf-search / cf-heur (双対の証明からの衝突のカット)
  方法: 1 CMIR、2 lifted knapsack cover、3 lifted mixed-integer cover、4 lifted mixed-binary cover、5 衝突 (cmir/cover が上書き)、6 含意、7 クリーク
- L 行: LP に入れたカット (submip, プールの行番号, プール番号, LP の行数, その時の LP の値)
列の種類は highspy で同じ模型を前処理して取る (前処理は HiGHS と同じ版)。
"""
import collections, json, sys
import highspy

h2 = highspy.Highs(); h2.setOptionValue("output_flag", False); h2.readModel(sys.argv[1]); h2.presolve()
plp = h2.getPresolvedLp()
integ = list(plp.integrality_); lo, up = list(plp.col_lower_), list(plp.col_upper_)
nrow, ncol = plp.num_row_, plp.num_col_
# 前処理後の行 (集約の行番号はこの番号)
A = plp.a_matrix_
rows = [[] for _ in range(nrow)]
if A.format_ == highspy.MatrixFormat.kColwise:
    st, ix, va = list(A.start_), list(A.index_), list(A.value_)
    for j in range(ncol):
        for k in range(st[j], st[j + 1]):
            rows[ix[k]].append((j, va[k]))
else:
    st, ix, va = list(A.start_), list(A.index_), list(A.value_)
    for i in range(nrow):
        rows[i] = [(ix[k], va[k]) for k in range(st[i], st[i + 1])]
rlo, rup = list(plp.row_lower_), list(plp.row_upper_)

def kind(j):
    if int(integ[j]) == 0: return "C"
    if lo[j] == 0 and up[j] == 1: return "B"
    return "I"

# 行の種類: 2 項の w-z の行 (VB)、データの行 (D: 一般整数列が 3 本以上)、その他
def rowkind(i):
    ks = collections.Counter(kind(j) for j, _ in rows[i])
    if len(rows[i]) == 2 and ks["B"] == 1 and ks["I"] == 1: return "VB"
    if ks["I"] >= 3: return "D"
    return "O" + "".join(f"{k}{ks[k]}" for k in "BIC" if ks[k])

# 「x0」相当: 一番多くの行に入る一般整数列
cnt = collections.Counter(j for i in range(nrow) for j, _ in rows[i])
x0 = max((j for j in range(ncol) if kind(j) == "I"), key=lambda j: cnt[j])
print(f"presolved {nrow} x {ncol}; kinds {collections.Counter(kind(j) for j in range(ncol))}; x0 = col {x0} in {cnt[x0]} rows, bounds [{lo[x0]}, {up[x0]}]")
print("row kinds", collections.Counter(rowkind(i) for i in range(nrow)).most_common(8))

P = []; L = []; last = {}
for line in open(sys.argv[2]):
    if line.startswith("P "):
        head, coefs, agg = line.rstrip("\n").split("|", 2)
        f = head.split()
        sub, pool, ri, src, meth, conf, rhs, ln = int(f[1]), int(f[2]), int(f[3]), f[4], int(f[5]), int(f[6]), float(f[7]), int(f[8])
        c = coefs.split(); cut = [(int(c[k]), float(c[k + 1])) for k in range(0, len(c), 2)]
        rec = dict(sub=sub, pool=pool, ri=ri, src=src, meth=meth, conf=conf, rhs=rhs, cut=cut, agg=agg.strip(), inlp=0, n=len(P))
        P.append(rec)
        if not sub: last[(pool, ri)] = rec
    elif line.startswith("L "):
        f = line.split(); sub, ci, pool, nr, obj = int(f[1]), int(f[2]), int(f[3]), int(f[4]), float(f[5])
        if sub: continue
        rec = last.get((pool, ci))
        L.append((rec, obj))
        if rec: rec["inlp"] += 1

def sig(cut):
    ks = collections.Counter(kind(j) for j, _ in cut)
    return "".join(f"{k}{ks[k]}" for k in "BIC" if ks[k])

def zonly(cut):
    return all(kind(j) == "B" for j, _ in cut)

main = [r for r in P if not r["sub"]]
print(f"\npool cuts (main solver) {len(main)}, submip {len(P) - len(main)}; LP adds {len(L)} (unmatched {sum(1 for r, _ in L if r is None)})")
tab = collections.Counter((r["src"], r["meth"]) for r in main)
tabz = collections.Counter((r["src"], r["meth"]) for r in main if zonly(r["cut"]))
tabl = collections.Counter((r["src"], r["meth"]) for r, _ in L if r)
tablz = collections.Counter((r["src"], r["meth"]) for r, _ in L if r and zonly(r["cut"]))
print("src meth : pool (z-only) / LP adds (z-only)")
for k, v in tab.most_common():
    print(f"  {k[0]:10s} {k[1]} : {v:6d} ({tabz[k]:5d}) / {tabl[k]:4d} ({tablz[k]:4d})")
# LP に入ったカットの形
print("\nLP-added cuts by shape:", collections.Counter(sig(r["cut"]) for r, _ in L if r).most_common(12))
print("pool z-only by src:", collections.Counter(r["src"] for r in main if zonly(r["cut"])))

# 集約の行の種類
def aggrows(agg):
    out = []
    for t in agg.split():
        if t.startswith("c:"): out.append(("cut", t))
        else:
            i, w = t.split(":"); out.append((int(i), float(w)))
    return out

nshow = int(sys.argv[3]) if len(sys.argv) > 3 else 3
shown = collections.Counter()
for r, obj in L:
    if r is None or not zonly(r["cut"]): continue
    key = (r["src"], r["meth"])
    if shown[key] >= nshow: continue
    shown[key] += 1
    print(f"\n=== LP z-cut src={r['src']} meth={r['meth']} conf={r['conf']} len={len(r['cut'])} rhs={r['rhs']} (LP obj {obj})")
    print("   cut:", " ".join(f"{v:+g}*{kind(j)}{j}" for j, v in r["cut"]), "<=", r["rhs"])
    ag = aggrows(r["agg"])
    print(f"   agg {len(ag)} rows:")
    for i, w in ag[:40]:
        if i == "cut": print("     ", w[:200]); continue
        print(f"     row {i} w={w:g} [{rowkind(i)}] {rlo[i]:g} <= " + " ".join(f"{a:+g}*{kind(j)}{j}" for j, a in rows[i][:14]) + (" ..." if len(rows[i]) > 14 else "") + f" <= {rup[i]:g}")
json.dump({"tab": [[list(k), v, tabz[k], tabl[k], tablz[k]] for k, v in tab.most_common()]}, open(sys.argv[2] + ".summ.json", "w"))
