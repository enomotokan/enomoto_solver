"""58 問 (60 秒・4 スレッド・種 1・2) の LU の修正の前 (luold) 対 後 (def) を benchlib.report で比べる。"""
import sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parent))
import benchlib as bl
R = bl.table(bl.load([here / "lu58.jsonl"])); keys = sorted(R)
bl.report(R, "luold", "def", keys, noise=bl.load_noise(), label="58 問 60 秒 (luold 対 def)")
for k in keys:
    a, b = R[k].get("luold"), R[k].get("def")
    if not a or not b: continue
    ta, tb = bl.t(a), bl.t(b)
    if bl.ok(a) != bl.ok(b) or (max(ta, tb) + 10) / (min(ta, tb) + 10) > 1.5:
        print(" ", k, "luold", a.get("status"), round(ta, 2), "def", b.get("status"), round(tb, 2))
    oa, ob = a.get("objective"), b.get("objective")
    if a.get("status") == "optimal" and b.get("status") == "optimal" and abs(oa - ob) > 1e-6 * (1 + abs(oa)):
        print("  MISMATCH", k, oa, ob)
