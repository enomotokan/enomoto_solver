"""共有の診断の集計: python summ.py > summ.txt。問題・設定ごとに種 1・2 を並べ、時間 (時間切れは上限、* 付き)、節点 N、
最後の木の lb≥opt の節点 (最終の暫定解で刈れた分)、木の段の 1 節点あたりのスレッド ms (4 × (T − R) / N)、潜りの LP の平均反復・ms、
待ち行列・共有から取ったノードの割合を出し、最後に設定ごとに def との比 (時間・節点・1 節点の手間の幾何平均) をまとめる。"""
import gzip, json, glob, math, re, collections
from pathlib import Path
D = Path(__file__).resolve().parent
import sys
sys.path.insert(0, str(D))
from cfg import CFG  # noqa: E402
def solved(r): return (r["result"].get("status") or "").lower() in ("optimal", "infeasible")
recs = {}
for f in sorted(glob.glob(str(D / "res_*_s*.jsonl"))):
    S = Path(f).name.split("_")[1]
    for l in open(f):
        if not l.strip(): continue
        r = json.loads(l); p, c, s = r["problem"], r["cfg"], r["seed"]
        T = min(r["result"].get("time") or r["tl"], r["tl"]) if solved(r) else r["tl"]
        lg = D / "logs" / f"{S}_{p}_{c}_s{s}.log.gz"
        lines = gzip.decompress(lg.read_bytes()).decode(errors="replace").splitlines() if lg.exists() else []
        trees = [float(m.group(1)) for x in lines for m in [re.match(r"TRSTAT: tree start outer ([0-9.]+)", x)] if m]
        lp = {k: [0, 0.0, 0.0] for k in ("dive", "queue", "pool")}; lbo = 0
        for x in lines:
            if x.startswith("TREEPAR: balance") and "| tid" in x:
                t = x.split("| tid", 1)[1]
                mm = re.search(r"lb>=opt (\d+)", t); lbo += int(mm.group(1)) if mm else 0
                for k in lp:
                    mm = re.search(k + r" (\d+) ([0-9.]+)it ([0-9.]+)ms", t)
                    if mm:
                        n, it, ms = int(mm.group(1)), float(mm.group(2)), float(mm.group(3)); lp[k][0] += n; lp[k][1] += n * it; lp[k][2] += n * ms
        R = min(trees[0], T) if trees else T; N = r["result"].get("nodes") or 0
        recs[(p, c, s)] = dict(T=T, ok=solved(r), N=N, lbo=lbo, per=4000 * max(T - R, 0) / N if N else float("nan"), lp=lp, ntr=len(trees), crash=(r["result"].get("status") or "").startswith("crash"))
probs = sorted({k[0] for k in recs}); seeds = sorted({k[2] for k in recs}); cfgs = [c for c in CFG if any(k[1] == c for k in recs)]
print(f"回 {len(recs)}、問題 {len(probs)}、設定 {cfgs}、種 {seeds}、crash {sum(m['crash'] for m in recs.values())}")
print("\n## 回ごと (T 秒・N 節点・lb≥opt・1 節点のスレッド ms・潜りの LP 反復/ms・待ち行列+共有の割合・木の数)")
for p in probs:
    for c in cfgs:
        for s in seeds:
            m = recs.get((p, c, s))
            if not m: continue
            n, it, ms = m["lp"]["dive"]; tot = sum(v[0] for v in m["lp"].values())
            q = 100 * (m["lp"]["queue"][0] + m["lp"]["pool"][0]) / tot if tot else 0
            print(f"{p:20s} {c:8s} s{s}: T {m['T']:6.1f}{'' if m['ok'] else '*'} N {m['N']:8d} lb>=opt {m['lbo']:7d} {m['per']:7.2f}ms 潜り {it/n if n else 0:6.1f}it {ms/n if n else 0:6.2f}ms 取り {q:4.1f}% 木 {m['ntr']}")
g = lambda v: math.exp(sum(math.log(max(x, 1e-9)) for x in v) / len(v)) if v else float("nan")
print("\n## 設定ごとの def との比 (問題 × 種の幾何平均。時間は 1 秒ずらし。解けた数は全回)")
for c in cfgs:
    rt, rn, rp, ok = [], [], [], 0
    per_prob = collections.defaultdict(list)
    for p in probs:
        for s in seeds:
            a, b = recs.get((p, "def", s)), recs.get((p, c, s))
            if not b: continue
            ok += b["ok"]
            if not a: continue
            rt.append((b["T"] + 1) / (a["T"] + 1)); per_prob[p].append((b["T"] + 1) / (a["T"] + 1))
            if a["N"] and b["N"]: rn.append(b["N"] / a["N"])
            if a["per"] == a["per"] and b["per"] == b["per"] and a["per"] > 0 and b["per"] > 0: rp.append(b["per"] / a["per"])
    pp = " ".join(f"{p[:10]} {g(v):.2f}" for p, v in per_prob.items())
    print(f"{c:8s}: 解けた {ok:2d} 時間 {g(rt):.3f} 節点 {g(rn):.3f} 1 節点 {g(rp):.3f} | {pp}")
