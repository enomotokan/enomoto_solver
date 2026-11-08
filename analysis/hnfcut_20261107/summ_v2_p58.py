"""58 問 (60 秒・4 スレッド・種 1・2) の既定 (def) 対 試作 (hcut_bc = ENOMOTO_MIP_HNFCUT=bc、改良 11-08) を benchlib.report で比べる。"""
import sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parent))
import benchlib as bl
R = bl.table(bl.load([here / "v2_p58.jsonl"]))
keys = sorted(R)
bl.report(R, "def", "hcut_bc", keys, noise=bl.load_noise(), label="58 問 60 秒")
# 片方だけ解けた・時間が 2 倍以上違う回
for k in keys:
    a, b = R[k].get("def"), R[k].get("hcut_bc")
    if not a or not b: continue
    ta, tb = bl.t(a), bl.t(b)
    if bl.ok(a) != bl.ok(b) or (max(ta, tb) + 10) / (min(ta, tb) + 10) > 1.5:
        print(" ", k, "def", a.get("status"), round(ta, 2), "hcut_bc", b.get("status"), round(tb, 2))
