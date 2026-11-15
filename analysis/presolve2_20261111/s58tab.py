"""epre.jsonl (58 問の洗い出し) を表にする: 問題ごとに既定と試作の根の LP の前の行・列・非零・根の LP。既定と変わった試作だけ印を付ける。"""
import json, collections, sys
from pathlib import Path
R = collections.defaultdict(dict)
for l in (Path(__file__).parent / (sys.argv[1] if len(sys.argv) > 1 else "epre.jsonl")).read_text().splitlines():
    r = json.loads(l); R[r["problem"]][r["cfg"]] = r
chg = collections.Counter()
for p, d in R.items():
    b = d.get("def", {})
    key = lambda r: (r.get("rows"), r.get("cols"), r.get("nnz"))
    out = []
    for c, r in d.items():
        if c == "def": continue
        if key(r) != key(b) or abs((r.get("root_lp") or 0) - (b.get("root_lp") or 0)) > 1e-6 * (1 + abs(b.get("root_lp") or 0)):
            chg[c] += 1
            out.append(f"{c} {r.get('rows')}×{r.get('cols')} nnz {r.get('nnz')} LP {r.get('root_lp')} ({r.get('t')}s)")
    print(f"{p}: 既定 {b.get('rows')}×{b.get('cols')} nnz {b.get('nnz')} LP {b.get('root_lp')} ({b.get('t')}s) | " + ("; ".join(out) or "同じ"))
print("変わった問題の数:", dict(chg))
