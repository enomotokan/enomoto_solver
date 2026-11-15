"""HiGHS のアブレーションの表 (hres.jsonl)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(D / "hres.jsonl")]
print("| 問題 | 設定 | 状態 | 秒 | ノード | 下界 | 解 |")
print("|---|---|---|---|---|---|---|")
for r in rows:
    st = "最適" if r["status"] == "Optimal" else "時間切れ"
    print(f"| {r['prob']} | {r['tag']} | {st} | {r['secs']:.1f} | {r['nodes']} | {r['bound']:.10g} | {r['obj']:.10g} |")
