"""9.75 節の 4: 1 節点の手間の増え方を parprof_20261117 のログで分ける。python item4.py > item4.txt
問題・種ごとに 1 スレッドと 4 スレッドで、木の段の 1 節点あたりのスレッド ms (全体)、その内の最初の LP の ms (潜り・待ち行列・共有の和を節点で割る)、
潜りの最初の LP の平均反復・平均 ms、遊び・大近傍探索の秒を節点で割った ms、木の中の再スタートの回数、最後の木の始まりの時刻 (外側の時計) を出す。"""
import gzip, json, glob, re, math
from pathlib import Path
PP = Path(__file__).resolve().parent.parent / "parprof_20261117"
def solved(r): return (r["result"].get("status") or "").lower() in ("optimal", "infeasible")
recs = {}
for f in sorted(glob.glob(str(PP / "res_*_s*.jsonl"))):
    S = Path(f).name.split("_")[1]
    for l in open(f):
        if not l.strip(): continue
        r = json.loads(l); p, th, s = r["problem"], r["threads"], r["seed"]
        if th not in (1, 4): continue
        T = min(r["result"].get("time") or r["tl"], r["tl"]) if solved(r) else r["tl"]
        lg = PP / "logs" / f"{S}_{p}_t{th}_s{s}.log.gz"
        lines = gzip.decompress(lg.read_bytes()).decode(errors="replace").splitlines() if lg.exists() else []
        trees = [float(m.group(1)) for l2 in lines for m in [re.match(r"TRSTAT: tree start outer ([0-9.]+)", l2)] if m]
        lp = {k: [0, 0.0, 0.0] for k in ("dive", "queue", "pool")}; idle = lns = 0.0; nodes_tp = 0
        for l2 in lines:
            if l2.startswith("TREEPAR: balance") and "| tid" in l2:
                t = l2.split("| tid", 1)[1]
                for key in ("idle", "tail"):
                    mm = re.search(key + r" ([0-9.]+)s", t); idle += float(mm.group(1)) if mm else 0
                mm = re.search(r"lns ([0-9.]+)s", t); lns += float(mm.group(1)) if mm else 0
                mm = re.search(r"nodes (\d+)", t); nodes_tp += int(mm.group(1)) if mm else 0
                for k in lp:
                    mm = re.search(k + r" (\d+) ([0-9.]+)it ([0-9.]+)ms", t)
                    if mm:
                        n, it, ms = int(mm.group(1)), float(mm.group(2)), float(mm.group(3)); lp[k][0] += n; lp[k][1] += n * it; lp[k][2] += n * ms
        R = min(trees[0], T) if trees else T; N = r["result"].get("nodes") or 0
        recs[(p, th, s)] = dict(T=T, R=R, N=N, lp=lp, idle=idle, lns=lns, ntr=len(trees), last=trees[-1] if trees else None, tl=r["tl"], ok=solved(r))
print("問題 種 | 1 節点 ms t1→t4 (比) | 最初の LP ms/節点 t1→t4 | 潜り LP 反復 t1→t4・ms t1→t4 | 待ち行列+共有 LP の割合 t1→t4 | 遊び+LNS ms/節点 t4 | 木の数 t1/t4 最後の木の始まり t1/t4")
rat = []
for (p, th, s) in sorted(k for k in recs if k[1] == 1):
    a, b = recs[(p, 1, s)], recs.get((p, 4, s))
    if not b or not a["N"] or not b["N"]: continue
    def per(m, k): return 1000 * k * max(m["T"] - m["R"], 0) / m["N"]
    def lpms(m): return sum(v[2] for v in m["lp"].values()) / m["N"]
    def dv(m): n, it, ms = m["lp"]["dive"]; return (it / n if n else 0, ms / n if n else 0)
    def qshare(m): tot = sum(v[0] for v in m["lp"].values()); return 100 * (m["lp"]["queue"][0] + m["lp"]["pool"][0]) / tot if tot else 0
    p1, p4 = per(a, 1), per(b, 4); ov = 1000 * (b["idle"] + b["lns"]) / b["N"]
    rat.append((p, s, p4 / p1 if p1 else float("nan")))
    print(f"{p:20s} s{s} | {p1:7.2f}→{p4:7.2f} ({p4/p1 if p1 else float('nan'):4.2f}) | {lpms(a):6.2f}→{lpms(b):6.2f} | {dv(a)[0]:6.1f}→{dv(b)[0]:6.1f} {dv(a)[1]:6.2f}→{dv(b)[1]:6.2f} | {qshare(a):4.1f}→{qshare(b):4.1f}% | {ov:6.2f} | {a['ntr']}/{b['ntr']} {a['last'] or 0:6.1f}/{b['last'] or 0:6.1f}")
