"""hnfb600.jsonl (600 秒) の表: 状態・秒・目的値・下界・ノード。"""
import json
from pathlib import Path
for l in (Path(__file__).resolve().parent / "hnfb600.jsonl").open():
    r = json.loads(l); x = r["result"]
    print(f"| {r['problem']} | {r['cfg']} | {r['rep']} | {x.get('status')} | {x.get('time', 0):.1f} | {x.get('objective')} | {x.get('bound')} | {x.get('nodes')} |")
