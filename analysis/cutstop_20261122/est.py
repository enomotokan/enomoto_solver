"""根のラウンドごとの得・損の見積り (同じ単位: 手間 ticks) をログ (CUTTK・TREETK) から作る。python est.py [問題_設定 ...] → est.txt に足す形で出す。

各ラウンド k (CUTTK はラウンドの初めの LP の値): 伸び Δz_k = z_{k+1} − z_k、手間 c_k = tk_{k+1} − tk_k。
損 = c_k + (nnz_{k+1} − nnz_k) / nnz_end × T_node × N  (足したカットで以後の 1 ノードが重くなる分。1 ノードの手間は LP の非零に比例とみなす)
得 = ΔN_k × T_node、ΔN_k = N × (1 − exp(−Δz_k / κ))、κ = G / ln N  (木の大きさが N ∝ exp(G / κ) とみなす。G は根の終わりの間隙 U − z_end)
T_node・N は事後の答え (TREETK の最後の 2 点の手間の差 / ノードの差、最終のノード数)。U は最終の暫定解の値 (なければ最適値)。
「止め時」= 得 < 損 になった最初のラウンド (その後も得 < 損が続くかは別に数える)。"""
import json, math, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
ref = {}
for l in open(WT / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="): ref[t[1]] = float(t[2])
res = {(r["problem"], r["cfg"]): r["result"] for r in map(json.loads, open(D / "diag.jsonl"))}


def parse(log):
    roots, cur, tree = [], None, []
    for l in open(log):
        if l.startswith("CUTTK"):
            m = re.search(r"round (\d+) obj (\S+) tk (\S+) rows (\d+) nnz (\d+) total_tk (\S+) secs (\S+)", l)
            k = int(m.group(1))
            if k == 0 or cur is None:
                cur = []; roots.append(cur)
            cur.append(dict(k=k, z=float(m.group(2)), tk=float(m.group(3)), rows=int(m.group(4)), nnz=int(m.group(5)), ttk=float(m.group(6)), secs=float(m.group(7))))
        elif l.startswith("CUTSTOP") and cur is not None:
            j = json.loads(l[len("CUTSTOP "):])
            cur.append(dict(stop=j.get("reason"), z_end=j.get("obj1")))
        elif l.startswith("TREETK"):
            m = re.search(r"nodes (\d+) secs (\S+) total_tk (\S+) lb (\S+) open (\d+)", l)
            tree.append(dict(n=int(m.group(1)), secs=float(m.group(2)), ttk=float(m.group(3)), lb=float(m.group(4))))
    return roots, tree


def est(name):
    p, c = name.rsplit("_", 1) if name.count("_") and name.rsplit("_", 1)[1] in ("def", "long", "cmu", "stall30") else (name, "")
    if name.endswith("_cmu_long"):
        p, c = name[: -len("_cmu_long")], "cmu_long"
    log = D / "logs" / f"{name}.log"
    roots, tree = parse(log)
    r = res.get((p, c), {})
    N = max(r.get("nodes") or 1, 2)
    U = r.get("objective") if r.get("objective") is not None else ref.get(p)
    if len(tree) >= 2:
        T_node = (tree[-1]["ttk"] - tree[-2]["ttk"]) / max(tree[-1]["n"] - tree[-2]["n"], 1)
    else:
        T_node = float("nan")
    out = [f"== {name}: status {r.get('status')} time {r.get('time', 0):.1f} nodes {N} U {U} T_node {T_node:.3g}"]
    for ri, rt in enumerate(roots):
        rounds = [x for x in rt if "k" in x]
        stop = next((x for x in rt if "stop" in x), {})
        if len(rounds) < 2 or U is None or not math.isfinite(T_node):
            out.append(f"  root {ri}: rounds {len(rounds)} stop {stop.get('stop')} (見積りなし)")
            continue
        z_end = stop.get("z_end") or rounds[-1]["z"]
        G = U - z_end
        nnz_end = rounds[-1]["nnz"]
        kappa = G / math.log(N) if G > 0 and N > 1 else float("inf")
        first_bad, n_bad, gsum, csum = None, 0, 0.0, 0.0
        rowsout = []
        for a, b in zip(rounds, rounds[1:]):
            dz = max(b["z"] - a["z"], 0.0)
            ck = b["tk"] - a["tk"]
            dnnz = b["nnz"] - a["nnz"]
            loss = ck + max(dnnz, 0) / nnz_end * T_node * N
            gain = N * (1 - math.exp(-dz / kappa)) * T_node if math.isfinite(kappa) and kappa > 0 else 0.0
            if gain < loss:
                n_bad += 1
                if first_bad is None: first_bad = a["k"]
            gsum += gain; csum += loss
            rowsout.append((a["k"], dz, ck, dnnz, gain, loss))
        last5 = rowsout[-5:]
        g5 = sum(x[4] for x in last5); l5 = sum(x[5] for x in last5)
        out.append(f"  root {ri}: rounds {len(rounds)} stop {stop.get('stop')} z {rounds[0]['z']:.6g} -> {z_end:.6g} G {G:.4g} | 得<損 {n_bad}/{len(rowsout)} 最初 {first_bad} | 全体 得/損 {gsum / max(csum, 1):.3g} 直近5巡 得/損 {g5 / max(l5, 1):.3g}")
    return "\n".join(out)


if __name__ == "__main__":
    names = sys.argv[1:] or sorted(f.stem for f in (D / "logs").glob("*.log"))
    print("\n".join(est(n) for n in names))
