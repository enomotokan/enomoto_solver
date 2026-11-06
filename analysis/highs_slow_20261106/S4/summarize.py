"""res.jsonl を問題・ラベルごとに表にする: python summarize.py"""
import json, re
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
rows = defaultdict(list)
for l in (D / "res.jsonl").read_text().splitlines():
    r = json.loads(l)
    lab = re.sub(r"_s\d+$", "", r["label"]) if r["solver"] == "ours" else re.sub(r"\d+$", "", r["label"])
    rows[(r["problem"], lab, r["tl"])].append(r)
for (p, lab, tl), rs in sorted(rows.items()):
    cells = []
    for r in rs:
        x = r["result"]; t = x.get("time") or 0
        st = "最適" if str(x.get("status")).lower() == "optimal" else "切れ"
        cells.append(f"{st} {t:.1f}秒 ({x.get('nodes')} ノード" + (f", 値 {x.get('objective')}, 下界 {x.get('bound')}" if st != "最適" else "") + ")")
    env = " ".join(f"{k}={v}" for k, v in rs[0]["env"].items())
    print(f"| {p} | {lab} | {'; '.join(cells)} | {env} | {tl} |")
