"""logs/ の診断の行を要約する: python summ_diag.py PREFIX [問題...]
各回: 結果、ノード数、ノードの LP の平均の行数・反復 (全スレッドの和から)、木の分離のラウンド・加えたラウンド・加えた行、
外した行 (木で加えた / ほか)、終わりの LP のカットの行 (うち木)、ノードの分離の秒、b・c で分離したノード、
木の下界の推移 (時刻ごとの最小の lb)。"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
pre = sys.argv[1]
rows = [json.loads(l) for l in open(D / f"{pre}.jsonl") if l.strip()]
want = set(sys.argv[2:])
num = r"([-\d.e+inf]+)"
pat = re.compile(r"TSDIAG tid (\d+) t ([\d.]+) nodes (\d+) lb (\S+) rows (\d+) cuts (\d+) treecuts (\d+)(?: postroot (\d+))? \| dt (\S+) lps (\d+) rows/lp (\S+) it/lp (\S+) cutsecs (\S+) \| ts rounds (\d+) added (\d+) rows (\d+) del tree (\d+) other (\d+) aged-tree (\d+)(?: forced (\d+))?")
for r in rows:
    p, c, s = r["problem"], r["cfg"], r["rep"]
    if want and p not in want:
        continue
    f = D / "logs" / f"{pre}_{p}_{c}_s{s}.log"
    if not f.exists():
        continue
    last = {}
    lps = rw = it = cs = 0.0
    lbt = []
    for l in f.read_text().splitlines():
        m = pat.match(l)
        if m:
            g = m.groups()
            tid = int(g[0])
            last[tid] = g
            n = int(g[9]); lps += n; rw += n * float(g[10]); it += n * float(g[11]); cs += float(g[12])
            lbt.append((float(g[1]), float(g[3])))
    if not last:
        continue
    tot = lambda k: sum(int(g[k] or 0) for g in last.values())
    res = r["result"]
    lbt.sort()
    # 10 個の時点での下界
    traj = []
    if lbt:
        T = lbt[-1][0]
        for q in (0.25, 0.5, 0.75, 1.0):
            v = [b for t, b in lbt if t <= q * T + 1e-9]
            traj.append(f"{q*T:.0f}s:{v[-1]:.6g}" if v else "-")
    print(f"{p:22s} {c:7s} s{s} {res.get('status','?')[:7]:7s} t {res.get('time') or 0:6.1f} gap-bound {res.get('bound')} obj {res.get('objective')}")
    print(f"    rows/lp {rw/max(lps,1):.1f} it/lp {it/max(lps,1):.1f} lps {int(lps)} | rounds {tot(13)} added {tot(14)} rows {tot(15)} del tree {tot(16)} other {tot(17)} aged-tree {tot(18)} forced {tot(19)} | end cuts {tot(5)} tree {tot(6)} postroot {tot(7)} cutsecs {cs:.1f}")
    print("    lb " + " ".join(traj))
