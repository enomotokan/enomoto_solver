"""res_t*.jsonl の GLOBALAUDIT の行を表にする: python analysis/globalaudit_20261124/tab.py res_t4.jsonl"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
f = D / (sys.argv[1] if len(sys.argv) > 1 else "res_t4.jsonl")
print("| 問題 | 時間 | ノード | 根の分枝 (再スタート回): 大域にない整数列の締め [理由] / うち固定 | 再スタートの判定 (今 / 局所の固定を足すと) | reset: 置いた締め付け → 捨てた伝播の整数列の締め (固定) | 衝突: 足した数 / 長さ 1 |")
print("|---|---|---|---|---|---|---|")
for r in map(json.loads, f.read_text().splitlines()):
    res = r["result"]
    roots, rst, cf = [], "", ""
    for l in r["ga"]:
        m = re.search(r"restart (\d+), trail \d+\): integer columns tighter than the global bounds (\d+) \[([^\]]*)\], of which fixed (\d+) \[([^\]]*)\].*?globally newly fixed integers (\d+), restart expected now (\w+) / with the local fixes (\w+)", l)
        if m:
            roots.append((m.group(1), m.group(2), m.group(3), m.group(4), m.group(6), m.group(7), m.group(8)))
        m = re.search(r"pending global tightenings (\d+) \(tightenings applied (\d+)\); dropped propagation would tighten integer bounds (\d+) \(fixing (\d+) integer columns\), continuous bounds (\d+), infeasible (\d+)", l)
        if m:
            rst = f"{m.group(2)} ({m.group(1)} 回) → {m.group(3)} ({m.group(4)})" + (f"、連続 {m.group(5)}" if m.group(5) != "0" else "") + (f"、矛盾 {m.group(6)}" if m.group(6) != "0" else "")
        m = re.search(r"literal conflicts added (\d+), of length 1 \(global fixings kept only in the pool\) (\d+) \(imported (\d+)\)", l)
        if m:
            cf = f"{m.group(1)} / {m.group(2)}" + (f" (取り込み {m.group(3)})" if m.group(3) != "0" else "")
    rt = "<br>".join(f"({a}) {b} [{c}] / {d}" for a, b, c, d, *_ in roots) or "根で分枝せず"
    rj = "<br>".join(f"({a}) 新固定 {e}: {f_} / {g}" for a, b, c, d, e, f_, g in roots) or "-"
    t = res.get("time")
    print(f"| {r['problem']} | {t:.1f} | {res.get('nodes')} | {rt} | {rj} | {rst or '-'} | {cf or '-'} |" if t is not None else f"| {r['problem']} | {res.get('status')} | | | | | |")
