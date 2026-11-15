"""modkvb_20261107 の既定 (def) の根のログ (ノード上限 1) を parse し、HiGHS の根の下界と並べる。出力: scan_existing.jsonl"""
import gzip, json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from parse import parse  # noqa: E402
MK = D.parent / "modkvb_20261107"
H = {}
for f in [MK / "root58.jsonl", MK / "rootmitt.jsonl", D.parent / "cmirdelta_20261107/root58.jsonl", D / "highs_root.jsonl"]:
    if f.exists():
        for l in open(f):
            r = json.loads(l)
            if r.get("cfg") == "highs" and r["result"].get("bound") is not None:
                H.setdefault(r["problem"], r["result"]["bound"])
out = open(D / "scan_existing.jsonl", "w")
for g in sorted((MK / "logs").glob("def_*_root.err.gz")):
    m = re.match(r"def_(.+)_(\d+)_r(\d+)_root\.err\.gz", g.name)
    p, tl, rep = m.group(1), int(m.group(2)), int(m.group(3))
    o = parse(gzip.open(g, "rt").read())
    o["rounds"] = {str(k): v for k, v in o["rounds"].items()}
    out.write(json.dumps({"problem": p, "tl": tl, "rep": rep, "highs": H.get(p), **o}) + "\n")
