"""最初の根 (再スタートの前) の RENS・RENS (more nodes)・RINS のサブ MIP 1 本ごとの秒とノードを設定ごとに平均する
(早めの再スタートで飛んだものは数えない)。使い方: python subroot.py [問題]"""
import re, sys, statistics as st
from pathlib import Path
D = Path(__file__).resolve().parent
prob = sys.argv[1] if len(sys.argv) > 1 else "neos-2978193-inde"
agg = {}
for f in sorted(D.glob(f"{prob}_*_*.log")):
    cfg = re.match(rf"{re.escape(prob)}_(.+)_\d+\.log$", f.name).group(1)
    first = f.read_text().split("MIP: restart 1 (")[0]
    lines = first.splitlines()
    for i, l in enumerate(lines):
        m = re.match(r"MIP:   parallel sub-MIP \(RENS\): status \w+, nodes (\d+), .*, ([\d.]+)s$", l)
        if m:
            agg.setdefault(cfg, {}).setdefault("RENS", []).append((float(m.group(2)), int(m.group(1))))
        m = re.match(r"MIP:   sub-MIP: \d+ free columns of \d+, status \w+, nodes (\d+), .*, ([\d.]+)s$", l)
        if m:
            nxt = next((x for x in lines[i + 1:i + 4] if x.startswith("MIP: root heuristic ")), "")
            k = "RENS+" if "RENS (more nodes)" in nxt else "RINS" if "RINS:" in nxt else None
            if k:
                agg.setdefault(cfg, {}).setdefault(k, []).append((float(m.group(2)), int(m.group(1))))
print(f"{'設定':12s} " + "  ".join(f"{k:>16s}" for k in ("RENS", "RENS+", "RINS")) + "   (本数 平均秒 / 平均ノード)")
for c in sorted(agg, key=lambda c: (c != "def", c != "cand", c)):
    print(f"{c:12s} " + "  ".join(f"{len(v):2d} {st.mean(t for t, _ in v):5.2f}s/{st.mean(n for _, n in v):5.0f}" if (v := agg[c].get(k)) else f"{'-':>16s}" for k in ("RENS", "RENS+", "RINS")))
