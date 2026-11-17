"""クリーク型の行の数え上げの集計: python summ.py > summ.txt。問題ごとに (1) クリーク型の行の数・割合・非零の割合、(2) 根の LP で余裕のある行の数・割合・非零の割合、
(3) 小さい行 (8 文字以下の不等式) を極大クリークでまとめられる組・減る行・非零の増減・破るクリークを足したときの下界の伸び、(4) 根の LP の反復・秒を並べ、
クリーク型の行の非零の割合で並べ替える。"""
import json, re
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(D / "res.jsonl") if l.strip()]
out = []
for r in rows:
    a = next((l for l in r["lines"] if "CLQROWS: rows" in l), None)
    b = next((l for l in r["lines"] if "CLQROWS merge" in l or "no small inequality" in l), None)
    if not a:
        out.append((r["problem"], None)); continue
    g = lambda pat, s: re.search(pat, s)
    m = g(r"rows (\d+) nnz (\d+); clique rows (\d+) \(([0-9.]+)% of rows, eq (\d+), len 2 / 3-8 / 9\+ = (\d+) / (\d+) / (\d+)\), nnz (\d+) \(([0-9.]+)%\); slack at the root LP (\d+) \(([0-9.]+)% of clique rows, nnz (\d+) = ([0-9.]+)% of all, eq (\d+), all-zero (\d+)\); root LP (\d+) iters ([0-9.]+)s", a)
    d = dict(zip("rows nnz ncl pcl neq l2 l38 l9 nnzcl pnnz slack pslack slnnz pslnnz sleq zero its secs".split(), m.groups()))
    d["grp"] = d["cov"] = d["viol"] = 0; d["gain"] = 0.0; d["nnzb"] = d["nnzc"] = 0; d["small"] = 0; d["grown"] = 0; d["lpst"] = "-"
    if b and "merge" in b:
        mm = g(r"small rows (\d+) -> (\d+) maximal cliques \((\d+) grown beyond the row\); groups of >= 2 rows (\d+) covering (\d+) rows \(rows -(\d+), nnz (\d+) -> (\d+)\); cliques violated by the root LP (\d+) \(in groups (\d+)\); LP with them \S+ -> \S+ \(gain (\S+) rel, (\d+) iters, (\S+)\)", b)
        if mm:
            x = mm.groups(); d.update(small=int(x[0]), grown=int(x[2]), grp=int(x[3]), cov=int(x[4]), nnzb=int(x[6]), nnzc=int(x[7]), viol=int(x[8]), gain=float(x[10]), lpst=x[12].rstrip(")"))
    out.append((r["problem"], d))
print(f"{len(rows)} 問 (CLQROWS なし: {[p for p, d in out if d is None]})\n")
print(f"{'問題':22s} {'行':>7s} {'クリーク行':>9s} {'割合':>6s} {'非零%':>6s} {'長さ 2/3-8/9+':>17s} {'等式':>6s} | {'余裕':>7s} {'%':>5s} {'非零%':>6s} | {'小':>6s} {'組':>5s} {'覆う行':>6s} {'非零 前→後':>15s} {'広がる':>6s} {'破る':>5s} {'伸び':>9s} | {'根 LP 反復':>9s} {'秒':>7s}")
for p, d in sorted(out, key=lambda t: -(float(t[1]["pnnz"]) if t[1] else -1)):
    if d is None: print(f"{p:22s} (CLQROWS なし)"); continue
    print(f"{p:22s} {d['rows']:>7s} {d['ncl']:>9s} {d['pcl']:>5s}% {d['pnnz']:>5s}% {d['l2']+'/'+d['l38']+'/'+d['l9']:>17s} {d['neq']:>6s} | {d['slack']:>7s} {d['pslack']:>4s}% {d['pslnnz']:>5s}% | "
          f"{d['small']:6d} {d['grp']:5d} {d['cov']:6d} {str(d['nnzb'])+'→'+str(d['nnzc']):>15s} {d['grown']:6d} {d['viol']:5d} {d['gain']:9.2e} | {d['its']:>9s} {d['secs']:>7s}")
