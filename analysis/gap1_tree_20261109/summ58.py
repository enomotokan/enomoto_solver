"""58 問 (60 秒、種 1・2) の def 対 試作 (tp2・uu) を benchlib の report で比べる。使い方: python summ58.py"""
import sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0]))
import benchlib as bl  # noqa: E402
R = bl.table(bl.load([here / "p58.jsonl"]))
keys = list(R)
for B in ["tp2", "uu"]:
    bl.report(R, "def", B, keys, noise=bl.load_noise(), label=f"58 問 60 秒 種 1・2 ({B})")
    rows = []
    for k in keys:
        a, b = R[k].get("def"), R[k].get(B)
        if a is None or b is None:
            continue
        ta, tb = bl.t(a), bl.t(b)
        if bl.ok(a) != bl.ok(b) or abs(ta - tb) > 0.3 * max(ta, tb) and max(ta, tb) > 5:
            rows.append((k, ta, tb, a.get("status"), b.get("status")))
    for k, ta, tb, sa, sb in sorted(rows):
        print(f"  {k[0]} 種 {k[1]}: def {ta:.1f} ({sa}) 対 {B} {tb:.1f} ({sb})")
    print()
