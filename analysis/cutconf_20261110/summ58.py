"""58 問 (60 秒、種 1・2) の def 対 試作 を benchlib の report で比べ、暫定解・下界の矛盾 (最適値・同じ問題のほかの回との) も調べる。
使い方: python summ58.py ファイル.jsonl 試作の設定 [時間上限]"""
import sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0]))
import benchlib as bl  # noqa: E402
f, B = sys.argv[1], sys.argv[2]
tl = float(sys.argv[3]) if len(sys.argv) > 3 else 60.0
bl.TL = tl
R = bl.table(bl.load([here / f]))
keys = [k for k in R if "def" in R[k] and B in R[k]]
bl.report(R, "def", B, keys, noise=bl.load_noise() if tl == 60.0 else None, label=f"{f} def 対 {B} ({len(keys)} 回)")
rows = []
for k in keys:
    a, b = R[k]["def"], R[k][B]
    ta, tb = bl.t(a, tl), bl.t(b, tl)
    if bl.ok(a, tl) != bl.ok(b, tl) or abs(ta - tb) > 0.3 * max(ta, tb) and max(ta, tb) > 5:
        rows.append((k, ta, tb, a.get("status"), b.get("status")))
for k, ta, tb, sa, sb in sorted(rows):
    print(f"  {k[0]} 種 {k[1]}: def {ta:.1f} ({sa}) 対 {B} {tb:.1f} ({sb})")
# 矛盾: 同じ問題の全部の回 (両方の設定) の中で、最小化として ある回の下界 > ほかの回の暫定解
from collections import defaultdict
P = defaultdict(list)
for (p, s), v in R.items():
    for c, r in v.items():
        P[p].append((c, s, r))
bad = 0
for p, L in P.items():
    # 問題は全部最小化 (.miplib_cache の MPS に OBJSENSE はない)
    for c1, s1, r1 in L:
        bd = r1.get("bound")
        if bd is None:
            continue
        for c2, s2, r2 in L:
            ob = r2.get("objective")
            if ob is None:
                continue
            if bd > ob + 1e-6 * max(1.0, abs(ob)) + 1e-9:
                bad += 1
                print(f"  矛盾: {p} {c1} 種 {s1} の下界 {bd} > {c2} 種 {s2} の暫定解 {ob}")
print("暫定解と下界の矛盾:", bad)
