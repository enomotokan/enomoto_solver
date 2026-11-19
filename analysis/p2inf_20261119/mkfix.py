"""P2-INF: 当方の前処理後の bnatt400 (列 C<j>) に、SCIP の分枝の記録の最初の飛び込みの道の決定 (深さ 1..k) を固定の境界として書き込み、
同じ部分木を当方と SCIP で解かせる問題を作る。python mkfix.py <前処理後の mps> <SCIP の分枝の記録 (tsv)> <k> <出力 mps>"""
import sys
src, tsv, k, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
# 最初の飛び込み: ノード番号 1 から、子に入るたびに深さが 1 増える列を、深さが初めて戻るところまでたどる
path, last_depth = [], 0
for l in open(tsv):
    num, par, dep, var, dr, b = l.rstrip("\n").split("\t")[:6]
    dep = int(dep)
    if dep != last_depth + 1:
        break
    path.append((var.replace("t_", ""), float(b)))
    last_depth = dep
    if len(path) >= k:
        break
lines = open(src).read().splitlines()
end = lines.index("ENDATA")
fx = [f" FX BND {v} {b:g}" for v, b in path]
if "BOUNDS" not in lines:
    lines.insert(end, "BOUNDS"); end += 1
lines[end:end] = fx
open(out, "w").write("\n".join(lines) + "\n")
print(len(path), "個を固定:", " ".join(f"{v}={b:g}" for v, b in path))
