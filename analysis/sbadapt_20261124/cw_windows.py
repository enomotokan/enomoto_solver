"""CW_RAMP の窓ごとの r (作った衝突 / ノード)・r̄ (前の値と半々の移動平均)・σ と、窓ごとの強分枝の LP の数・伝播で切った子の数を、
診断 SBADAPT_FEAT の行 (200 ノードごと、累計の比) から出す (sbadapt_20261124 6 節)。
使い方 (リポジトリの根から): python3 analysis/sbadapt_20261124/cw_windows.py <名札> [問題=30n20b8] [窓の数=6]"""
import gzip, math, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
tag = sys.argv[1]; p = sys.argv[2] if len(sys.argv) > 2 else "30n20b8"; k = int(sys.argv[3]) if len(sys.argv) > 3 else 6
rows = []
for l in gzip.open(D / "logs" / f"{tag}_{p}.log.gz", "rt"):
    if l.startswith("SBADAPT_FEAT"):
        t = l.split(); d = dict(zip(t[1::2], t[2::2]))
        n = int(d["nodes"])
        # cpn = multi_stats[1] / (nodes + 1)、sbpn = sb_lps / nodes、sbinf = 伝播で切った子 / sb_lps
        rows.append((n, float(d["cpn"]) * (n + 1), float(d["sbpn"]) * n, float(d["sbinf"]) * float(d["sbpn"]) * n))
print("窓の終わり  r      r̄      σ      重み    強分枝LP  伝播で切った子")
prev = (0, 0.0, 0.0, 0.0); ema = -1.0
for r in rows[:k]:
    dn = r[0] - prev[0]
    rr = (r[1] - prev[1]) / dn
    ema = rr if ema < 0 else 0.5 * ema + 0.5 * rr
    s = 1 / (1 + math.exp(-(ema - 0.69) / 0.1))
    print(f"{r[0]:8d}  {rr:5.2f}  {ema:5.2f}  {s:5.3f}  {0.1*s:6.4f}  {r[2]-prev[2]:8.0f}  {r[3]-prev[3]:8.1f}")
    prev = r
