#!/usr/bin/env python3
"""logs/*.log と *.json から、根のヒューリスティクスの終わりの時刻・裏のカットのラウンド数と目的値・根の下界・結果を表にする。
使い方: python3 summary.py [名前の接頭辞 ...]"""
import json, re, sys
from pathlib import Path

d = Path(__file__).parent / "logs"
pre = sys.argv[1:]
print("| 回 | 結果 | 時間 | ノード | 根のヒューリスティクスの終わり | 裏のカット (ラウンド・目的値) | 根の下界 | 根の RENS | 根の RINS | 最初の木の RINS の解 |")
print("|---|---|---|---|---|---|---|---|---|---|")
for js in sorted(d.glob("*.json")):
    name = js.stem
    if pre and not any(name.startswith(p) for p in pre):
        continue
    try:
        r = json.loads(js.read_text())
    except Exception:
        continue
    log = (d / f"{name}.log").read_text()
    m = re.search(r"after root heuristics: incumbent Some\(([\d.e+-]+)\) \(([\d.]+)s\)", log)
    heur = f"{m.group(2)} 秒 ({float(m.group(1)):.0f})" if m else "-"
    m = re.search(r"background cuts: (\d+) rounds, .*-> ([\d.e+-]+) \(waited ([\d.]+)s", log)
    bg = f"{m.group(1)}・{float(m.group(2)):.1f} (待ち {m.group(3)})" if m else "-"
    m = re.search(r"parallel sub-MIP \(RENS\): status (\w+), nodes (\d+), objective Some\(([\d.]+)\), ([\d.]+)s", log)
    rens = f"{m.group(1)} {m.group(2)} ノード {float(m.group(3)):.0f} {m.group(4)} 秒" if m else "-"
    m = re.search(r"sub-MIP: \d+ free columns of \d+, status (\w+), nodes (\d+), objective Some\(([\d.]+)\), ([\d.]+)s\nMIP: root heuristic RINS", log)
    rins = f"{m.group(1)} {m.group(2)} ノード {float(m.group(3)):.0f} {m.group(4)} 秒" if m else "-"
    m = re.search(r"new incumbent ([\d.e+-]+) at node (\d+) \(([\d.]+)s\) \[(?:other thread: )?tree RINS\]", log)
    tr = f"{float(m.group(1)):.0f} ({m.group(3)} 秒)" if m else "-"
    rs = len(re.findall(r"restart", log, re.I))
    st = r["status"] + (f" {r['objective']:.0f}" if r["status"] != "optimal" and r.get("objective") else "")
    print(f"| {name} | {st} | {r['time']:.1f} | {r['nodes']} | {heur} | {bg} | {r.get('root_bound', 0):.1f} | {rens} | {rins} | {tr} |" + (f" 再スタート {rs}" if rs else ""))
