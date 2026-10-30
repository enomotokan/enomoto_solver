#!/usr/bin/env python3
"""logs/*.log と *.json から、根のヒューリスティクスの終わりの時刻・待ち・裏のカットのラウンド数と目的値・根の下界・結果を表にする。
使い方: python3 summary.py [名前の接頭辞 ...]"""
import json, re, sys
from pathlib import Path

d = Path(__file__).parent / "logs"
pre = sys.argv[1:]
print("| 回 | 結果 | 時間 | ノード | 根のヒューリスティクスの終わり | 待ち (上限・理由) | 裏のカット (ラウンド・目的値) | 置き換えの終わり | 根の下界 |")
print("|---|---|---|---|---|---|---|---|---|")
for js in sorted(d.glob("*.json")):
    name = js.stem
    if pre and not any(name.startswith(p) for p in pre):
        continue
    try:
        r = json.loads(js.read_text())
    except Exception:
        continue
    log = (d / f"{name}.log").read_text()
    m = re.search(r"after root heuristics: incumbent (?:Some\(([\d.e+-]+)\)|None) \(([\d.]+)s\)", log)
    heur = f"{m.group(2)} 秒" if m else "-"
    m = re.search(r"background cuts: waited ([\d.]+)s for progress \(cap ([\d.]+)s, (\w+)\)", log)
    wt = f"{m.group(1)} ({m.group(2)}・{m.group(3)})" if m else "-"
    m = re.search(r"background cuts: (\d+) rounds, .*-> ([\d.e+-]+) \(waited ([\d.]+)s, ([\d.]+)s\)", log)
    if m:
        bg, end = f"{m.group(1)}・{float(m.group(2)):.1f}", f"{m.group(4)} 秒"
    else:
        m = re.search(r"background cuts: (?:(\d+) rounds, obj [\d.e+-]+ not above|no result)", log)
        bg, end = ("使わず" if m else "-"), "-"
    st = r["status"] + (f" {r['objective']:.6g}" if r["status"] != "optimal" and r.get("objective") is not None else "")
    rb = r.get("root_bound")
    print(f"| {name} | {st} | {r['time']:.1f} | {r['nodes']} | {heur} | {wt} | {bg} | {end} | {rb if rb is None else f'{rb:.6g}'} |")
