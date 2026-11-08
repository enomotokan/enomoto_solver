"""ENOMOTO のログ (logs/*.err.gz) の要約: 下界・暫定解の推移 (1/4 ごと)、刈り込みの理由 (ENOMOTO_MIP_TREE_LOG の ENODE 行)、
分枝・衝突・伝播・再スタートの数。使い方: python summ_e.py <ログ>..."""
import gzip, re, sys
from collections import Counter
for fn in sys.argv[1:]:
    op = gzip.open if fn.endswith(".gz") else open
    prog, reasons, stats = [], Counter(), []
    restarts = 0
    nodecnt = []
    with op(fn, "rt", errors="replace") as f:
        for l in f:
            if l.startswith("ENODE"):
                m = re.search(r"r=(\w*)", l); reasons[m.group(1) if m else "?"] += 1; continue
            m = re.match(r"MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)", l)
            if m:
                prog.append((float(m.group(1)), int(m.group(2)), int(m.group(3)), m.group(4), m.group(5))); continue
            if "restart" in l and l.startswith("MIP: restart"):
                restarts += 1
            if l.startswith(("MIP: dual proofs", "MIP: conflict analysis (HiGHS", "MIP: literal conflicts", "MIP: cut pool propagation", "MIP: branching:", "MIP: tree search threads", "MIP: objective propagation", "MIP: symmetry", "MIP: orbit")):
                stats.append(l.strip()[:260])
            if l.startswith("MIP: NODECNT"):
                nodecnt.append(l.strip())
    print("==", fn)
    if prog:
        T = prog[-1][0]
        for q in (0.25, 0.5, 0.75, 1.0):
            r = min(prog, key=lambda p: abs(p[0] - q * T))
            print(f"  t={r[0]:.0f} nodes={r[1]} open={r[2]} lb={r[3]} ub={r[4]}")
    if reasons:
        print("  ENODE:", dict(reasons.most_common()))
    print("  restart lines:", restarts)
    for s in stats[-12:]:
        print("  ", s)
    for s in nodecnt:
        print("  ", s)
