#!/usr/bin/env python3
"""logs/ の各回のログから、状態・時間・ノード数・従の join の最大の待ち・gdb のスタックの有無の表を作る。

使い方: python3 summary.py [logs のディレクトリ]
ログの名前は 接頭辞_問題_上限_回.log (ab.sh は ab_on/ab_off_問題_上限_回.log)。
"""
import json
import re
import sys
from pathlib import Path

d = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).parent / "logs"
rows = []
for f in sorted(d.glob("*.log")):
    m = re.match(r"(.+?)_([A-Za-z0-9.\-]+?)_(\d+)_(\d+)$", f.stem)
    if not m:
        continue
    tag, prob, lim, rep = m.groups()
    text = f.read_text(errors="replace")
    res = {}
    for line in reversed(text.splitlines()):
        if line.startswith("{"):
            try:
                res = json.loads(line)
            except json.JSONDecodeError:
                pass
            break
    joins = [float(x) for x in re.findall(r"joined ([0-9.]+)s after the stop signal", text)]
    gdb = f.with_suffix(".gdb").exists()
    t = res.get("time", float("nan"))
    rows.append((tag, prob, int(lim), int(rep), res.get("status"), t, res.get("nodes"), max(joins, default=0.0), t - int(lim), gdb))

print("| 設定 | 問題 | 上限 | 回 | 状態 | 時間 | ノード | join の最大 | 上限との差 | スタック |")
print("|---|---|---:|---:|---|---:|---:|---:|---:|---|")
for r in rows:
    print(f"| {r[0]} | {r[1]} | {r[2]} | {r[3]} | {r[4]} | {r[5]:.2f} | {r[6]} | {r[7]:.2f} | {r[8]:+.2f} | {'あり' if r[9] else ''} |")
