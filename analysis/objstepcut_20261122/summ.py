"""root.jsonl (根だけ、1 スレッド) の表: 刻みの行を作った問題について、既定・a・b の根の下界 (刻みに切り上げた値も)・ラウンド・LP 反復・止まり方・時間。
python summ.py > root.txt"""
import json, math
from collections import defaultdict
R = defaultdict(dict)
for l in open("root.jsonl"):
    r = json.loads(l); R[r["problem"]][r["mode"]] = r
print("| 問題 | g | ⌈z/g⌉ (最初→最後, a) | 既定: 下界・ラウンド・反復・止まり方・秒 | a | b |")
print("|---|---|---|---|---|---|")
for p, d in R.items():
    if not all(m in d for m in "0ab") or not d["a"]["objstep"]["n"]: continue
    def f(r):
        cs = r["cutstop"][-1] if r["cutstop"] else None
        b = r["root_bound"]
        return f"{b:.6g} ・{r['rounds']}・{r['cut_lp_iters']}・{cs[0] if cs else '-'}・{(r['time'] or 0):.1f}" if b is not None else str(r["status"])
    o = d["a"]["objstep"]
    print(f"| {p} | {o['g']} | {o['first'][1]} → {o['last'][1]} | {f(d['0'])} | {f(d['a'])} | {f(d['b'])} |")
