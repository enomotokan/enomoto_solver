"""b58o (次元 d3・d2・d1) でも b58n と同じく、 b58l/nb5final.py の設定名を一般化した写し (「獲得」の欄は設定名)。
=5 (解きながらのベイズ最適化) の最終値の集計 (analysis/treeclose_20261114.md 9.23 節、親経由の利用者の質問)。
python nb5final.py [ログの glob ...] (既定 logs/*_nb5*.log.gz)。ログの最後の "MIP: NODESEL_BANDIT5:" 行から
(1) GP の最良点・実際に選んだ値の手間 (LP の反復) で重み付けた平均・最後の 1/4 の区間の平均、
(2) 同じ問題 × 獲得関数の種どうしの最良点の差、(3) 既定・d3・d1 のどれに近いか、を表にし、
(4) 問題ごとに 3 値の時系列の図を svg/<問題>.svg に書く。
軌跡の各項は PQ/SQ/u:報酬[@反復]。@反復のない古い形式 (86ff22ba5 まで、600 文字で切れて最初の約 25 区間だけ) は重みを 1 とする。"""
import glob, gzip, math, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
files = sys.argv[1:] or sorted(glob.glob(str(D / "logs" / "*.log.gz")))
# 錨の点 (PLUNGE_QUOT, SIBLING_QUOT, u)。u は潜りのノード数の上限 min(nodes/10^(1-u), 10^(2+7u)) の指数。NB5_INIT と同じ
ANCH = {"既定": (0.25, 0.25, 1 / 7), "d3": (1.0, 1.0, 1 / 7), "d1": (1.0, 1.0, 1.0)}
def x(p):  # 探索空間 [0,1]^3 の座標
    return ((p[0] - 0.25) / 0.75, (p[1] - 0.25) / 0.75, p[2])
def dist(a, b):
    return math.dist(x(a), x(b))
def near(p):
    return min(ANCH, key=lambda k: dist(p, ANCH[k]))
pat = re.compile(r"intervals (\d+), GP best point PLUNGE_QUOT ([\d.]+) SIBLING_QUOT ([\d.]+) u ([\d.]+) \(predicted reward ([^)]+)\), trajectory (.*)")
R = []
for f in files:
    m = re.match(r"(.+)_([a-z0-9]+)_s(\d+)\.log(\.gz)?$", Path(f).name)
    if not m:
        continue
    line = None
    with (gzip.open(f, "rt", errors="replace") if f.endswith(".gz") else open(f, errors="replace")) as fh:
        for l in fh:
            if "NODESEL_BANDIT5:" in l:
                line = l
    if not line:
        continue
    g = pat.search(line)
    k = int(g[1]); best = (float(g[2]), float(g[3]), float(g[4]))
    tr = []
    for t in g[6].split():
        a, _, rest = t.partition(":")
        r, _, w = rest.partition("@")
        tr.append((tuple(map(float, a.split("/"))), float(r), float(w) if w else 1.0))
    if not tr:
        continue
    W = sum(w for *_, w in tr)
    wm = tuple(sum(p[i] * w for p, _, w in tr) / W for i in range(3))
    q = tr[-max(1, len(tr) // 4):]
    lq = tuple(sum(p[i] for p, _, _ in q) / len(q) for i in range(3))
    R.append(dict(prob=m[1], acq={"nb5u": "UCB", "nb5e": "EI"}.get(m[2], m[2]), seed=int(m[3]), k=k, best=best, wm=wm, lq=lq, tr=tr,
                  full=len(tr) >= k - 1))
f3 = lambda p: f"{p[0]:.2f}/{p[1]:.2f}/{p[2]:.2f}"
print("## (1)(3) 問題 × 獲得関数 × 種 (値は PLUNGE_QUOT/SIBLING_QUOT/u、近いのは既定 0.25/0.25/0.14・d3 1/1/0.14・d1 1/1/1 のどれか)")
print("| 問題 | 獲得 | 種 | 区間 (軌跡) | GP 最良点 | 近い | 手間重み平均 | 近い | 最後の 1/4 平均 | 近い |")
print("|---|---|---|---|---|---|---|---|---|---|")
for r in sorted(R, key=lambda r: (r["prob"], r["acq"], r["seed"])):
    print(f"| {r['prob']} | {r['acq']} | {r['seed']} | {r['k']} ({len(r['tr'])}{'' if r['full'] else ' 切れ'}) | {f3(r['best'])} | {near(r['best'])} | "
          f"{f3(r['wm'])} | {near(r['wm'])} | {f3(r['lq'])} | {near(r['lq'])} |")
print("\n## (2) 種 1・2 の最良点の差 (探索空間 [0,1]^3 の距離、対角線は 1.73) と近い錨が揃うか")
print("| 問題 | 獲得 | 最良点 種1 | 最良点 種2 | 距離 | 錨 揃う |")
print("|---|---|---|---|---|---|")
by = {}
for r in R:
    by.setdefault((r["prob"], r["acq"]), {})[r["seed"]] = r
for (p, a), v in sorted(by.items()):
    if 1 in v and 2 in v:
        b1, b2 = v[1]["best"], v[2]["best"]
        print(f"| {p} | {a} | {f3(b1)} | {f3(b2)} | {dist(b1, b2):.2f} | {'○' if near(b1) == near(b2) else '×'} ({near(b1)}/{near(b2)}) |")
from collections import Counter
print("\n## 錨ごとの数 (GP 最良点 / 手間重み平均 / 最後の 1/4)")
for key in ("best", "wm", "lq"):
    print(f"- {key}: {dict(Counter(near(r[key]) for r in R))}")
# (4) 図: 問題ごとに、行 = 獲得 × 種、各行に 3 値の時系列 (横は区間の番号、手間の累積)
out = D / "svg"; out.mkdir(exist_ok=True)
COL = {0: "#1f77b4", 1: "#d62728", 2: "#2ca02c"}
NAMES = ["PLUNGE_QUOT", "SIBLING_QUOT", "u"]
probs = sorted({r["prob"] for r in R})
for p in probs:
    rs = sorted([r for r in R if r["prob"] == p], key=lambda r: (r["acq"], r["seed"]))
    Wd, H, L = 720, 110, 150
    svg = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{Wd + L + 20}" height="{H * len(rs) + 50}" font-family="sans-serif" font-size="11">',
           '<rect width="100%" height="100%" fill="white"/>', f'<text x="10" y="16" font-size="13">{p}: =5 の区間ごとの選んだ値 (横は LP の反復の累積)</text>']
    for i, (j, n) in enumerate(COL.items()):
        svg.append(f'<text x="{L + 200 * i}" y="34" fill="{n}">■ {NAMES[j]}</text>')
    for row, r in enumerate(rs):
        y0 = 45 + row * H; h = H - 25
        tot = sum(w for *_, w in r["tr"]); cum = 0
        svg.append(f'<text x="10" y="{y0 + h / 2}">{r["acq"]} 種 {r["seed"]}</text><text x="10" y="{y0 + h / 2 + 14}" fill="#666">区間 {r["k"]}{"" if r["full"] else " (軌跡切れ)"}</text>')
        svg.append(f'<rect x="{L}" y="{y0}" width="{Wd}" height="{h}" fill="none" stroke="#aaa"/>')
        for v in (0.25, 1.0):
            yy = y0 + h * (1 - v)
            svg.append(f'<line x1="{L}" x2="{L + Wd}" y1="{yy:.1f}" y2="{yy:.1f}" stroke="#eee"/><text x="{L - 28}" y="{yy + 4:.1f}" fill="#999">{v}</text>')
        pts = {j: [] for j in COL}
        for pnt, _, w in r["tr"]:
            xa, xb = L + Wd * cum / tot, L + Wd * (cum + w) / tot; cum += w
            for j in COL:
                yy = y0 + h * (1 - pnt[j]) + (j - 1) * 1.5
                pts[j] += [f"{xa:.1f},{yy:.1f}", f"{xb:.1f},{yy:.1f}"]
        for j, n in COL.items():
            svg.append(f'<polyline fill="none" stroke="{n}" stroke-width="1.5" points="{" ".join(pts[j])}"/>')
    svg.append("</svg>")
    (out / f"{p}.svg").write_text("\n".join(svg))
print(f"\n図: {out}/<問題>.svg ({len(probs)} 枚)")
