"""run.py のログを表にする (2026-11-01)。使い方: python summ.py。"""
import json, re
from pathlib import Path
D = Path(__file__).resolve().parent
print("| 形 | 回 | 時間 | 状態 | 最適値 (暫定解) | 最後の下界 | 再スタート | 木に入る時刻 | RENS やり直し 走った/省いた | 根の RENS (並列) | 最後の暫定解 (時刻・出どころ) | ノード |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|")
for c in ("real", "real_d2lp1"):
    for rep in range(6):
        f = D / f"{c}_{rep}.log"
        if not f.exists():
            continue
        t = f.read_text()
        res = json.loads([l for l in t.splitlines() if l.startswith("{")][-1])
        lbs = re.findall(r"nodes\s+\d+ open\s+\d+ lb (\S+) ub", t)
        rst = t.count("MIP: root LP after the restart")
        tree = re.findall(r"tree search with \d+ threads \(([\d.]+)s\)", t)
        ran = len(re.findall(r"root heuristic RENS \(more nodes\)", t))
        skip = len(re.findall(r"RENS \(more nodes\) skipped", t))
        inc = re.findall(r"new incumbent (\S+) at node \d+ \(([\d.]+)s\) \[([^\]]+)\]", t)
        last = f"{float(inc[-1][0]):.2f} @{inc[-1][1]} {inc[-1][2]}" if inc else "-"
        prens = re.findall(r"parallel sub-MIP \(RENS\): status (\w+), nodes (\d+)", t)
        print(f"| {c} | {rep} | {res['time']:.1f} | {res['status']} | {res.get('objective')} | {lbs[-1] if lbs else '-'} | {rst} | "
              f"{', '.join(tree) or '-'} | {ran}/{skip} | {'; '.join(f'{s} {n}' for s, n in prens) or '-'} | {last} | {res.get('nodes')} |")
