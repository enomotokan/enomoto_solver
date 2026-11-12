"""対象の 9 問の表: HiGHS の前処理後 (rules.jsonl の既定) と、ENOMOTO の根の LP の前の大きさ (既定と試作、epre_targets.jsonl・epre.jsonl)。"""
import json, collections
from pathlib import Path
D = Path(__file__).parent
H = {}
for l in (D / "rules.jsonl").read_text().splitlines():
    r = json.loads(l)
    if r.get("off") == 0 and "rows" in r: H[r["problem"]] = r
E = collections.defaultdict(dict)
for f in ("epre_targets.jsonl", "epre.jsonl"):
    for l in (D / f).read_text().splitlines():
        r = json.loads(l)
        if r["problem"] in H and "rows" in r: E[r["problem"]].setdefault(r["cfg"], r)
CF = ["def", "spl", "prr0", "int", "pcl", "dom", "enum", "deq4", "all"]
print("| 問題 | HiGHS 行×列 | " + " | ".join(CF) + " |")
print("|---|---|" + "---|" * len(CF))
for p, h in H.items():
    cells = []
    for c in CF:
        r = E[p].get(c)
        cells.append(f"{r['rows']}×{r['cols']} ({r['root_lp']:.6g})" if r else "")
    print(f"| {p} | {h['rows']}×{h['cols']} | " + " | ".join(cells) + " |")
