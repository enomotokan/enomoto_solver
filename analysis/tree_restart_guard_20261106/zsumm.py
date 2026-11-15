"""改良 (11-08) の 600 秒の 5 問 (z_ の札) の表: 時間・下界・暫定解・相対ギャップ・再スタートの回数・ガードが止めた回数。
使い方: python zsumm.py [札の頭 (既定 z_)]"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
pre = sys.argv[1] if len(sys.argv) > 1 else "z_"
rows = defaultdict(dict)
for l in open(D / "res.jsonl"):
    r = json.loads(l)
    if not r["tag"].startswith(pre):
        continue
    s = r["result"]
    ub, lb = s.get("objective"), s.get("bound")
    gap = abs(ub - lb) / max(abs(ub), 1) if ub is not None and lb is not None else float("nan")
    log = D / "logs" / f"{r['tag']}_{r['problem']}_{r['seed']}.log"
    txt = log.read_text(errors="replace") if log.exists() else ""
    npass = len(re.findall(r"TRSTAT: (?:due|inc) .*guard pass", txt))
    stops = re.findall(r"TRSTAT: (?:due|inc) .*guard stop (\S)", txt)
    cell = (f"{s['time']:.1f}" if s["status"] == "optimal" else f"TL gap {gap:.2e} lb {lb:.8g} ub {ub:.8g}") + (f" [再 {npass}・止 {len(stops)}]" if r["tag"] != pre + "def" else "")
    rows[(r["problem"], r["seed"])][r["tag"]] = cell
tags = sorted({t for v in rows.values() for t in v}, key=lambda t: (t != pre + "def", t))
print("| 問題 | 種 | " + " | ".join(tags) + " |")
print("|---|---|" + "---|" * len(tags))
for (p, sd) in sorted(rows):
    print(f"| {p} | {sd} | " + " | ".join(rows[(p, sd)].get(t, "-") for t in tags) + " |")
