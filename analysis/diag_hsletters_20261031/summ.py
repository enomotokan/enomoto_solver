"""run.jsonl を問題 × 設定の表にする (時間 (秒)、時間切れは T)。"""
import json, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
t = collections.defaultdict(list); cfgs = []; probs = []
for l in open(D / (sys.argv[1] if len(sys.argv) > 1 else "run.jsonl")):
    r = json.loads(l); o = json.loads(r["out"]) if r["out"].startswith("{") else {}
    s = f"{o['time']:.1f}" if o.get("status") == "optimal" else ("T" if o else "?")
    t[(r["prob"], r["cfg"])].append(s)
    if r["cfg"] not in cfgs: cfgs.append(r["cfg"])
    if r["prob"] not in probs: probs.append(r["prob"])
print("| 問題 | " + " | ".join(cfgs) + " |"); print("|---" * (len(cfgs) + 1) + "|")
for p in probs:
    print(f"| {p} | " + " | ".join("・".join(t[(p, c)]) for c in cfgs) + " |")
