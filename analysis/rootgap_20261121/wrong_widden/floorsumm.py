"""floorcnt_SET.jsonl の集計: 下限あり (1) で働いた回数と、根の下界・状態が下限なし (0) と変わった問題。python floorsumm.py SET"""
import json, sys
from collections import defaultdict
SET = sys.argv[1]
d = defaultdict(dict)
for l in open(f"floorcnt_{SET}.jsonl"):
    r = json.loads(l); d[r["problem"]][r["floor"]] = r
n = len(d); fired = []; diff = []
for p, v in sorted(d.items()):
    a, b = v.get("0"), v.get("1")
    if not (a and b): continue
    st = b["stats"] or [0, 0, 0, 0]
    if any(st): fired.append((p, st))
    ra, rb = a.get("root_bound"), b.get("root_bound")
    ch = a["status"] != b["status"] or (ra is None) != (rb is None) or (ra is not None and abs(ra - rb) > 1e-6 * (1 + abs(ra)))
    if ch: diff.append((p, a["status"], b["status"], ra, rb, st))
print(f"{SET}: 問題 {n}、下限が働いた問題 {len(fired)}、根の下界か状態が変わった問題 {len(diff)}")
print("働いた [吸収した係数, そのカット, 全部消えて捨てた, 無限の境界で捨てた]:")
for p, st in fired: print(" ", p, st)
print("変わった (下限なし → あり、根の下界):")
for x in diff: print(" ", x)
