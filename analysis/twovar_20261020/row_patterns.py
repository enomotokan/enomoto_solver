"""行の形 (係数を並べた型と右辺) の頻度: row_patterns.py <mps> [長さ] [only.txt]
0-1 列だけの片側の行を <= の向きにし、係数を降順に並べた (係数..., 右辺) で数える。"""
import sys
from collections import Counter
sys.path.insert(0, __import__("os").path.dirname(__file__))
from twovar_census import load  # noqa: E402

path = sys.argv[1]
L = int(sys.argv[2]) if len(sys.argv) > 2 else 3
n, m, rows, rl, ru, binary, lo, up, integ = load(path)
c = Counter()
for i, r in enumerate(rows):
    if len(r) != L:
        continue
    tag = "B" if all(binary[j] for j, _ in r) else "M"
    if rl[i] == ru[i]:
        c[(tag, "E", tuple(sorted(round(a, 3) for _, a in r)), round(ru[i], 3))] += 1
        continue
    for sg, b in ((1, ru[i]), (-1, -rl[i])):
        if abs(b) < 1e30:
            co = tuple(sorted((round(sg * a, 3) if binary[j] else ("c", round(sg * a, 3)) for j, a in r), key=str))
            c[(tag, "L", co, round(b, 3))] += 1
for k, v in c.most_common(25):
    print(v, k)
