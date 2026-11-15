"""res.jsonl を表にする (問題 x 設定: 状態・時間・暫定解 / 下界・ノード数)。使い方: python summ.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(D / "res.jsonl")]
print("| 問題 | ソルバー | 設定 | 状態 | 時間 | 暫定解 | 下界 | ノード |")
print("|---|---|---|---|---|---|---|---|")
for r in sorted(rows, key=lambda r: (r["problem"], r["solver"] != "highs", r["cfg"])):
    x = r["result"]
    f = lambda v: "-" if v is None else (f"{v:.6g}" if isinstance(v, float) else str(v))
    print(f"| {r['problem']} | {r['solver']} | {r['cfg']} | {x.get('status')} | {f(x.get('time'))} | {f(x.get('objective'))} | {f(x.get('bound'))} | {x.get('nodes')} |")
