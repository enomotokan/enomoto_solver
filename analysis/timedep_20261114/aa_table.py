"""1 スレッドの A/A の表: python aa_table.py 札の組 (例 n,s:tn,ts) 問題,...
各 (問題, 札) の 秒/ノード、最大/最小の比 (shift 1)、回どうしが最後まで同じ道か (firstdiff で「同じ」か RESULT 行だけの違い)。"""
import re, sys, subprocess, math
from pathlib import Path
D = Path(__file__).resolve().parent
TH = __import__("os").environ.get("AA_TH", "1")
groups = [g.split(",") for g in sys.argv[1].split(":")]; probs = sys.argv[2].split(",")
def res(f):
    r = [l for l in open(f, errors="replace") if l.startswith("RESULT")]
    if not r: return None
    p = r[-1].split(); return (p[1], float(p[6]), int(p[4]))
def same(a, b):
    out = subprocess.run([sys.executable, str(D / "firstdiff.py"), a, b, "0"], capture_output=True, text=True).stdout
    return out.startswith("同じ") or ("A> RESULT" in out)
print("| 問題 | " + " | ".join("/".join(g) for g in groups) + " |"); print("|---|" + "---|" * len(groups))
for p in probs:
    cells = []
    for g in groups:
        runs = {t: sorted(D.glob(f"logs/{p}_t{TH}_{t}[0-9].log")) for t in g}
        txt = []
        for t, fs in runs.items():
            rs = [(f, res(f)) for f in fs if res(f)]
            if not rs: continue
            ts = [r[1] for _, r in rs]
            ratio = (max(ts) + 1) / (min(ts) + 1)
            ident = all(same(str(rs[0][0]), str(f)) for f, _ in rs[1:]) if len(rs) > 1 else None
            s = " · ".join(f"{'T' if r[0] != 'optimal' else ''}{r[1]:.1f}/{r[2]}" for _, r in rs)
            txt.append(f"{t}: {s} (×{ratio:.2f}{', 同じ道' if ident else ''})")
        cells.append("<br>".join(txt))
    print(f"| {p} | " + " | ".join(cells) + " |")
