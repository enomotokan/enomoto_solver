"""diag.jsonl の表 (根の LP を列だけの基底から / 基底なし (同時実行) で)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
f = sys.argv[1] if len(sys.argv) > 1 else "diag.jsonl"
print("| 問題 | 渡す (既定) | 固定 | 前処理の LP 行→根 (非零) | 中身の違う行 | 基底の列 kb / 固定 | 特異 | 主実行不能 | 前処理の最初の LP 反復 / 秒 | 列だけの基底から 秒 (反復) | 基底なし 秒 (反復) | 比 | 目的値の差 |")
print("|---|---|---:|---|---:|---|---|---:|---|---|---|---:|---|")
for l in open(D / f):
    r = json.loads(l); d = r["diag"] or {}
    fe = d.get("feat") or {}; w = d.get("warm") or {}; c = d.get("cold") or {}
    if not fe:
        print(f"| {r['problem']} | (前処理の LP の基底なし) | | | | | | | | | {c.get('t')} ({c.get('it')}) {c.get('st')} | | |"); continue
    chg = 1 - fe["same_rows"] / max(fe["m1"], 1)
    ratio = (w.get("t") or 0) / max(c.get("t") or 1e-9, 1e-3)
    dob = "" if w.get("obj") is None or c.get("obj") is None else f"{abs(w['obj'] - c['obj']) / (1 + abs(c['obj'])):.1e}"
    pinf = "-" if fe["pinf_cnt"] is None else f"{fe['pinf_cnt']} ({fe['pinf_cnt'] / fe['m1']:.1%})"
    print(f"| {r['problem']} | {'渡す' if fe['passed'] else '渡さない'} | {fe['n_fixed']} | {fe['m0']}→{fe['m1']} ({fe['nnz0']}→{fe['nnz1']}) | {chg:.1%} | {fe['kb']} / {fe['kb_fixed']} | {'特異' if fe['singular'] else '-'} | {pinf} | {fe['probe_iters']} / {fe['probe_secs']:.1f} | {w.get('t')} ({w.get('it')}) {'' if w.get('st') == 'Optimal' else w.get('st')} | {c.get('t')} ({c.get('it')}) {'' if c.get('st') == 'Optimal' else c.get('st')} | {ratio:.2f} | {dob} |")
