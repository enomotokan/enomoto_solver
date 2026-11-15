"""big.jsonl とログから 6 問の表 (根の LP、ノードの LP、ノード、下界、解、双子の数) を作る。"""
import json, re
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / "big.jsonl").read_text().splitlines()]
def g(pat, s, k=1):
    m = re.search(pat, s)
    return m.group(k) if m else "-"
for r in rows:
    log = (D / "logs" / f"big_{r['cfg']}_{r['problem']}_{r['rep']}.log").read_text(errors="replace")
    root = g(r"MIP: root LP [-0-9.e+]+ \((\d+ iters, [0-9.]+s)\)", log)
    nd = re.findall(r"node diag: nodes (\d+) node LPs (\d+) .*?iters (\d+) \(([0-9.]+)/LP\) LP secs ([0-9.]+)", log)
    ndt = f"{nd[-1][3]} 反復/LP, {float(nd[-1][4]) / max(1, int(nd[-1][1])) * 1000:.0f} ms/LP" if nd else "-"
    res = r["result"]
    ls = [l for l in log.splitlines() if l.startswith("LPSCALE: lps")]
    print(f"| {r['problem']} | {r['cfg']}{'' if r['rep'] == 0 else ' (回 ' + str(r['rep']) + ')'} | {root} | {ndt} | {res.get('nodes')} | {res.get('bound')} | {res.get('objective')} |")
    if ls:
        print("   ", ls[-1])
