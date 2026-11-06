"""res.jsonl を 問題 × 設定 の時間の表にする (種ごとに / で区切る。時間切れは TL)。使い方: python pivot.py [ファイル]"""
import json, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
f = sys.argv[1] if len(sys.argv) > 1 else D / "res.jsonl"
t = collections.defaultdict(lambda: collections.defaultdict(list))
tags = []
for l in open(f):
    r = json.loads(l); res = r["result"]
    v = f"{res.get('time') or 0:.1f}" if res.get("status") == "optimal" else f"TL"
    t[r["problem"]][r["tag"]].append(f"{v}(s{r['seed']})")
    if r["tag"] not in tags: tags.append(r["tag"])
print("| 問題 | " + " | ".join(tags) + " |")
print("|---" * (len(tags) + 1) + "|")
for p, d in t.items():
    print(f"| {p} | " + " | ".join(" ".join(d.get(g, ["-"])) for g in tags) + " |")
