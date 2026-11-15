"""表: 根の LP を二段解法だけで解いた結果 (base.jsonl・scale.jsonl) と HiGHS の双対単体法 (highs_off.jsonl: 尺度の調整あり、highs_noscale.jsonl: なし)。"""
import json
from pathlib import Path
H = Path(__file__).resolve().parent
def load(f, key="problem"):
    p = H / f
    return {json.loads(l)[key]: json.loads(l) for l in p.open()} if p.exists() else {}
B, S, HO, HN = load("base.jsonl"), load("scale.jsonl"), load("highs_off.jsonl"), load("highs_noscale.jsonl")
def ours(r):
    if not r:
        return "-"
    st = r["st"]
    us = 1e6 * r["t"] / max(1, r["iters"])
    tag = "" if st == "Optimal" else f" ({st})"
    return f"{r['t']:.1f} / {r['iters']} / {us:.0f}{tag}"
def hi(r):
    if not r:
        return "-"
    tag = "" if r["status"] == "Optimal" else " (時間切れ)"
    return f"{r['t']:.1f} / {r['iters']} / {1e6 * r['t'] / max(1, r['iters']):.0f}{tag}"
print("| 問題 | 行 | 列 | 非零 | 二段 既定 (秒 / 反復 / µs/反復) | 二段 SCALE | HiGHS 尺度あり | HiGHS 尺度なし |")
print("|---|---:|---:|---:|---|---|---|---|")
for p, r in B.items():
    print(f"| {p} | {r['m']} | {r['n']} | {r['nnz']} | {ours(r)} | {ours(S.get(p))} | {hi(HO.get(p))} | {hi(HN.get(p))} |")
