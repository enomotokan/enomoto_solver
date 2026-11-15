"""scan.jsonl から条件に当たる問題を数える。使い方: python summarize_scan.py [行の閾値 幅の閾値]"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
M = float(sys.argv[1]) if len(sys.argv) > 1 else 1e4
R = float(sys.argv[2]) if len(sys.argv) > 2 else 1e4
c = json.load(open(D / "candidates.json")); grp = {p: "bm" for p in c["bench_mitt"]} | {p: "coll" for p in c["coll_sample"]}
rs = [json.loads(l) for l in open(D / "scan.jsonl")]
hit, noev = [], []
for r in rs:
    if not r["events"]: noev.append(r["problem"]); continue
    h = [e for e in r["events"] if e["rows"] >= M and e["range"] >= R]
    if h: hit.append((r["problem"], grp.get(r["problem"]), [(e["rows"], f'{e["range"]:.1e}', e["warm"]) for e in r["events"]], r["root_lp"]))
print(f"scanned {len(rs)}, hits {len(hit)} (M>={M:g}, range>={R:g}), no events {len(noev)}: {noev}")
for h in hit: print(*h)
