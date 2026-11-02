"""results.jsonl とログから表を作る (2026-11-02)。使い方: python summarize.py  (このディレクトリを基準に読む)"""
import json, re
from pathlib import Path
from collections import defaultdict
D = Path(__file__).parent
rows = [json.loads(l) for l in open(D / "results.jsonl") if l.strip()]
modes = ["def", "fj", "fl", "fl31"]
by = defaultdict(list)
for r in rows:
    by[(r["problem"], r["mode"])].append(r)
def early(p, m, k):
    f = D / "logs" / f"{p}_{m}_r{k}.log"
    if not f.exists():
        return ""
    s = f.read_text(errors="replace")
    a = re.search(r"early FJ for the feasibility problem: .*found (\w+) \(([\d.]+)s", s)
    if a:
        return ("FJ" if a.group(1) == "true" else "なし") + f" {a.group(2)}"
    a = re.search(r"early FJ/Local-MIP for the feasibility problem: .*found (\w+) by ([\w-]+) \(([\d.]+)s", s)
    if a:
        return (a.group(2) if a.group(1) == "true" else "なし") + f" {a.group(3)}"
    a = re.search(r"new incumbent \S+ at node \d+ \(([\d.]+)s\) \[([^\]]+)\]", s)
    if a:
        return f"(木) {a.group(2)} {a.group(1)}"
    return "-"
probs = list(dict.fromkeys(r["problem"] for r in rows))
print("| 問題 | " + " | ".join(modes) + " |")
print("|---|" + "---|" * len(modes))
for p in probs:
    cells = []
    for m in modes:
        rs = sorted(by[(p, m)], key=lambda r: r["run"])
        if not rs:
            cells.append(""); continue
        ok = sum(r.get("status") == "optimal" for r in rs)
        ts = " / ".join(f"{r['time']:.1f}" if r.get("status") == "optimal" else ("切" if r.get("status") == "time_limit" else r.get("status", "?")) for r in rs)
        cells.append(f"{ok}/{len(rs)}: {ts}")
    print(f"| {p} | " + " | ".join(cells) + " |")
print()
print("早めの探索の結果 (見つけた方と時刻、既定は最初の暫定解)")
print("| 問題 | " + " | ".join(modes) + " |")
print("|---|" + "---|" * len(modes))
for p in probs:
    cells = []
    for m in modes:
        rs = sorted(by[(p, m)], key=lambda r: r["run"])
        cells.append(" / ".join(early(p, m, r["run"]) for r in rs))
    print(f"| {p} | " + " | ".join(cells) + " |")
print()
print("時間切れの回のノード数 (見つからないときの損の目安。早めの探索の 6 秒は木の時間を減らす)")
print("| 問題 | " + " | ".join(modes) + " |")
print("|---|" + "---|" * len(modes))
for p in probs:
    cells = []
    for m in modes:
        rs = sorted(by[(p, m)], key=lambda r: r["run"])
        cells.append(" / ".join(str(r.get("nodes")) for r in rs if r.get("status") == "time_limit"))
    if any(cells):
        print(f"| {p} | " + " | ".join(cells) + " |")
