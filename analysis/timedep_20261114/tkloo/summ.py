"""TICKS の文字ごとの切り分けのまとめ: python summ.py > summ.txt
問題ごとに、設定ごとの時間 (時間切れは 60 秒) の幾何平均 (shift 1 秒) と cand1117 比、種ごとの時間を並べる。
neos-827175 は logs/ のログから、段ごとの時刻 (根の LP の秒、最後のカットのラウンド・finish start の時刻 (前処理の後から)、前処理を含む求解全体) の中央値も出す。"""
import gzip, json, glob, math, re, statistics, collections
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_s*.jsonl"))) for l in open(f) if l.strip()]
def t(r):
    res = r["result"]
    return min(res["time"], r["tl"]) if res.get("status") in ("optimal", "infeasible") else r["tl"]
T = collections.defaultdict(dict)
for r in rows: T[(r["problem"], r["cfg"])][r["seed"]] = t(r)
cfgs = list(dict.fromkeys(r["cfg"] for r in rows))
from cfg import CFGS
cfgs = [c for c in CFGS if any((p, c) in T for p in ("neos-827175", "10teams"))]
print(f"行数 {len(rows)}")
for p in ("neos-827175", "10teams"):
    g = {c: math.exp(statistics.mean(math.log(v + 1) for v in T[p, c].values())) - 1 for c in cfgs if T.get((p, c))}
    if not g: continue
    print(f"\n## {p} (幾何平均 shift 1 秒、cand1117 比、種 1〜8 の秒)")
    for c in cfgs:
        if c in g: print(f"  {c:12s} {g[c]:6.1f}  {g[c] / g['cand1117']:.2f}  {[round(T[p, c].get(s, float('nan')), 1) for s in range(1, 9)]}")
STAGES = [("root LP", r"MIP: root LP -?[\d.e+-]+ \(\d+ iters, ([\d.]+)s\)"), ("last cut round", r"MIP: cut round \d+: .*obj [-\d.e+]+ \(([\d.]+)s\)"),
          ("finish", r"MIP: finish start \w+ \(([\d.]+)s\)"), ("solve", r"MIP: solve returned \w+ after ([\d.]+)s")]
print("\n## neos-827175 の根の段 (各設定の種の中央値、秒。ログにない段は -)")
for c in cfgs:
    v = collections.defaultdict(list)
    for f in glob.glob(str(D / "logs" / f"{c}_neos-827175_s*.log.gz")):
        txt = gzip.open(f, "rt").read()
        for name, rx in STAGES:
            m = re.findall(rx, txt)
            if m: v[name].append(float(m[-1]))
    print(f"  {c:12s} " + "  ".join(f"{n} {statistics.median(v[n]):.2f}" if v[n] else f"{n} -" for n, _ in STAGES))
