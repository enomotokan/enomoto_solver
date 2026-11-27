"""30n20b8 (打ち切り値つき) の切り分けの回の終わりの統計を並べる (sbadapt_20261124 6 節)。
使い方 (リポジトリの根から): python3 analysis/sbadapt_20261124/harm_stats.py [名札 ...]"""
import gzip, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
tags = sys.argv[1:] or ["bcut", "pbcut", "pbnocutcut", "pbnccut", "pbnocutbtcut", "pbbtnccut", "pbside", "spside", "pbnb"]
pat = {
    "nodes": r"MIP:\s+[\d.]+s nodes\s+(\d+)",
    "secs": r"MIP:\s+([\d.]+)s nodes",
    "sbprop_cut": r"strong branching children infeasible by propagation (\d+)",
    "hull": r"node bounds from both children (\d+)",
    "sel": r"branching: (\d+) selections",
    "sb_lps": r"sb LPs (\d+)",
    "proofs": r"dual proofs and conflicts (\d+)",
    "farkas": r"Farkas (\d+)",
    "proof_pruned": r"dual proofs and conflicts .*pruned (\d+) nodes",
    "prop_conf": r"propagation conflicts: analysed (\d+)",
    "ca_calls": r"conflict analysis \(HiGHS style\): calls (\d+)",
    "ca_added": r"conflict analysis \(HiGHS style\): calls \d+, conflicts \d+, added (\d+)",
    "ca_len": r"conflict analysis \(HiGHS style\):.*mean length ([\d.]+)",
    "lit_added": r"literal conflicts: added (\d+)",
    "lit_pool": r"literal conflicts: added \d+, in pool (\d+)",
    "lit_pruned": r"literal conflicts: .*pruned (\d+) nodes",
    "lit_tight": r"literal conflicts: .*tightened (\d+) bounds",
    "bumped": r"bumped columns per analysis ([\d.]+)",
    "cw_mean": r"cw ramp diag: .*mean weight ([\d.e+-]+)",
    "pc_obs": r"positive gains (\d+) of (\d+) observations",
    "prop_ticks": r"MIP: TICKS .* prop (\d+)",
}
print("名札", *pat.keys(), sep="\t")
for t in tags:
    f = D / "logs" / f"{t}_30n20b8.log.gz"
    if not f.exists():
        continue
    s = gzip.open(f, "rt").read()
    row = [t]
    for k, r in pat.items():
        m = re.findall(r, s)
        v = m[-1] if m else "-"
        row.append("/".join(v) if isinstance(v, tuple) else v)
    print(*row, sep="\t")
