"""(b) 58 問 (60 秒、種 1・2) の def 対 試作を benchlib の report で比べる。使い方: python summ58.py [設定,...]"""
import sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0]))
import benchlib as bl  # noqa: E402
R = bl.table(bl.load([here / "p58.jsonl"]))
keys = list(R)
for B in (sys.argv[1].split(",") if len(sys.argv) > 1 else ["bc", "bcA"]):
    ks = [k for k in keys if "def" in R[k] and B in R[k]]
    if not ks:
        continue
    bl.report(R, "def", B, ks, noise=bl.load_noise(), label=f"58 問 60 秒 種 1・2 ({B})")
    rows = []
    for k in ks:
        a, b = R[k]["def"], R[k][B]
        ta, tb = bl.t(a), bl.t(b)
        if bl.ok(a) != bl.ok(b) or abs(ta - tb) > 0.3 * max(ta, tb) and max(ta, tb) > 5:
            rows.append((k, ta, tb, a.get("status"), b.get("status"), a.get("nodes"), b.get("nodes")))
    for k, ta, tb, sa, sb, na, nb in sorted(rows):
        print(f"  {k[0]} 種 {k[1]}: def {ta:.1f} ({sa}, {na} ノード) 対 {B} {tb:.1f} ({sb}, {nb} ノード)")
    print()
