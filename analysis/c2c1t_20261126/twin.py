"""同じ強分枝の子を、伝播の境界つき (SB_PROBEBD) と境界なしで解いた伸び (単位あたり) の比べ (README 4 節、診断 ENOMOTO_MIP_SB_PROP_TWIN の PTW の行)。
python twin.py pclog/n85_twin_s*.log.gz"""
import gzip, math, statistics, sys
for f in sys.argv[1:]:
    rows = []
    for l in gzip.open(f, "rt"):
        t = l.split()
        if t[0] != "PTW": continue
        _, nodes, depth, j, up, gp, g0, st, nres = t
        rows.append((int(depth), float(gp), float(g0), st, int(nres)))
    eff = [r for r in rows if r[4] > 0 and r[3] == "Optimal"]
    same = sum(1 for r in rows if r[4] == 0)
    big = [r for r in eff if r[1] > r[2] * (1 + 1e-9) + 1e-9]
    ratios = [r[1] / r[2] for r in eff if r[2] > 1e-9]
    z_to_pos = sum(1 for r in eff if r[2] <= 1e-9 and r[1] > 1e-9)
    print(f"== {f.split('/')[-1]}: 子 {len(rows)} (伝播で締まった列がない子 {same})、伝播が効いた子 {len(eff)}")
    print(f"   伝播つきの伸びのほうが大きい {len(big)}、境界なしで 0 なのに伝播つきで正 {z_to_pos}")
    if ratios: print(f"   両方正の子の比 (伝播つき / なし): 中央 {statistics.median(ratios):.1f}、最大 {max(ratios):.1f} ({len(ratios)} 子)")
    print(f"   伸びの合計: 伝播つき {sum(r[1] for r in eff):.3e}、なし {sum(r[2] for r in eff):.3e}")
