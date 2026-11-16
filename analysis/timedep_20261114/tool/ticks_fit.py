"""手間の単位の重みの当てはめ (analysis/timedep_20261114.md の 11 節): python tool/ticks_fit.py logs_tool/<札>
ログの "MIP: TICKS cpu S ... lp a prop b ..." (外側の求解、主のスレッド) と、サブ MIP の行の "TICKS cpu S lp a ..." を標本にし、
cpu ≈ Σ w_k × 手間_k を相対誤差で当てはめる (非負の最小二乗、各標本を cpu で割る)。cpu 0.05 秒未満の標本は除く。
重み (秒 / 手間) と、当てはめた秒と実の秒の比の分布 (問題ごとのずれ) を出す。"""
import re, sys, glob, math
import numpy as np
from scipy.optimize import nnls
NAMES = ["lp", "prop", "sep", "pre", "heur", "node", "lpit", "lu", "lum"]
# `--outer`: 外側の求解の標本だけ。`--w` w1,...,w9 (ns): 当てはめずに与えた重みで比を出す (別の組のログに当てる確かめ)
OUTER = "--outer" in sys.argv
WGIVEN = [float(v) * 1e-9 for v in sys.argv[sys.argv.index("--w") + 1].split(",")] if "--w" in sys.argv else None
X, y, src = [], [], []
for f in sorted(glob.glob(sys.argv[1] + "/*.log")):
    for l in open(f, errors="ignore"):
        m = re.search(r"TICKS cpu ([\d.]+) (?:total [\d.]+ )?(.*?)(?: \(|$)", l.strip())
        if not m: continue
        cpu = float(m.group(1)); kv = dict(re.findall(r"(\w+) (\d+)", m.group(2)))
        if cpu < 0.05 or not all(k in kv for k in NAMES): continue
        if OUTER and "MIP: TICKS" not in l: continue
        X.append([float(kv[k]) for k in NAMES]); y.append(cpu); src.append((f.split("/")[-1], "outer" if "MIP: TICKS" in l else "sub"))
X, y = np.array(X), np.array(y)
A = X / y[:, None]
w = np.array(WGIVEN) if WGIVEN else nnls(A, np.ones(len(y)))[0]
pred = X @ w
r = pred / y
print(f"標本 {len(y)} (外側 {sum(s[1] == 'outer' for s in src)}・サブ MIP {sum(s[1] == 'sub' for s in src)})")
print("重み (ns / 手間):", {k: round(v * 1e9, 3) for k, v in zip(NAMES, w)})
print("当てはめ / 実の秒: 中央値 %.2f、10%% %.2f、90%% %.2f、log の標準偏差 %.2f" % (np.median(r), np.percentile(r, 10), np.percentile(r, 90), np.std(np.log(r))))
for name, w1 in [("LP だけ", None)]:
    w_lp = np.median(y / np.maximum(X[:, 0], 1)); r2 = X[:, 0] * w_lp / y
    print("比べ: LP の手間だけ (中央値の重み %.3e): 中央値 %.2f、10%% %.2f、90%% %.2f、log の標準偏差 %.2f" % (w_lp, np.median(r2), np.percentile(r2, 10), np.percentile(r2, 90), np.std(np.log(r2))))
bad = sorted(zip(r, src, y), key=lambda t: abs(math.log(t[0])))[-10:]
print("ずれの大きい標本 (比, 問題, 種類, 秒):", [(round(a, 2), b[0], b[1], round(c, 2)) for a, b, c in bad])
