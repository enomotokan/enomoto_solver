"""揺れの表: python summ.py [res_t1.jsonl res_t4.jsonl ...]  (既定は両方)
問題ごとに、時間 (時間切れは TL のまま、shift 1 秒) の最大/最小・幾何標準偏差 (gsd)・解けた割合、並べ替えだけ (種 1 の 4 回) と
種だけ (並べ替え 0 の 3 回) の最大/最小、時間切れの回の主ギャップ (最適値は解けた回の目的値) を出す。
解けた回の目的値が問題の中で食い違えば「食い違い」に出す (並べ替えは目的値を変えないはず)。"""
import json, math, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
files = sys.argv[1:] or [str(D / "res_t1.jsonl"), str(D / "res_t4.jsonl")]
SH = 1.0
def gsd(ts):
    l = [math.log(t + SH) for t in ts]
    if len(l) < 2: return 1.0
    m = sum(l) / len(l)
    return math.exp(math.sqrt(sum((x - m) ** 2 for x in l) / (len(l) - 1)))
def pgap(o, ref):
    if o is None or ref is None: return 1.0
    if abs(o - ref) <= 1e-9 * max(1, abs(ref)): return 0.0
    if o * ref < 0: return 1.0
    return abs(o - ref) / max(abs(o), abs(ref))
def load(f):
    rows = collections.defaultdict(dict)
    if not Path(f).exists(): return rows
    for l in open(f):
        if l.strip():
            r = json.loads(l); rows[r["problem"]][(r["perm"], r["seed"])] = r
    return rows
def stats(runs, tl):
    def t(r):
        res = r["result"]
        return min(res.get("time") or tl, tl) if res.get("status") == "optimal" else tl
    ts = {v: t(r) for v, r in runs.items()}
    ok = {v: runs[v]["result"].get("status") == "optimal" for v in runs}
    allt = list(ts.values())
    pt = [ts[v] for v in ts if v[1] == 1]
    st = [ts[v] for v in ts if v[0] == 0]
    mm = lambda a: (max(a) + SH) / (min(a) + SH) if a else float("nan")
    return ts, ok, dict(n=len(allt), solved=sum(ok.values()), mm=mm(allt), gsd=gsd(allt), mm_perm=mm(pt), mm_seed=mm(st),
                        tmin=min(allt), tmax=max(allt), gm=math.exp(sum(math.log(x + SH) for x in allt) / len(allt)) - SH)
out = []
for f in files:
    rows = load(f)
    if not rows: continue
    th = next(iter(next(iter(rows.values())).values()))["threads"]
    out.append(f"## {th} スレッド ({Path(f).name}、{sum(len(v) for v in rows.values())} 回)\n")
    mism, crash, table = [], [], []
    for p, runs in rows.items():
        tl = next(iter(runs.values()))["tl"]
        objs = [r["result"]["objective"] for r in runs.values() if r["result"].get("status") == "optimal"]
        crash += [(p, v) for v, r in runs.items() if r["result"].get("status") not in ("optimal", "time_limit")]
        if objs and max(objs) - min(objs) > 1e-6 * max(1, abs(objs[0])): mism.append((p, min(objs), max(objs)))
        ref = objs[0] if objs else None
        ts, ok, s = stats(runs, tl)
        gaps = [pgap(r["result"].get("objective"), ref) for v, r in runs.items() if not ok[v]] if ref is not None else []
        nosol = sum(1 for v, r in runs.items() if not ok[v] and r["result"].get("objective") is None)
        seq = " ".join(("T" if not ok[v] else "") + f"{ts[v]:.1f}" for v in sorted(ts))
        table.append((s["gsd"], p, s, seq, gaps, nosol))
    out.append(f"食い違い (解けた回の目的値): {mism or 'なし'}、crash など: {crash or 'なし'}\n")
    out.append("| 問題 | 解けた | 最小〜最大 (秒) | 最大/最小 | gsd | 並べ替えだけ | 種だけ | 回ごと (p0s1 p0s2 p0s3 p1s1 p2s1 p3s1) | 時間切れの回の主ギャップ・解なし |")
    out.append("|---|---|---|---|---|---|---|---|---|")
    for g, p, s, seq, gaps, nosol in sorted(table, key=lambda x: (-x[0], x[1])):
        gp = " ".join(f"{x:.2g}" for x in gaps) + (f" 解なし{nosol}" if nosol else "")
        out.append(f"| {p} | {s['solved']}/{s['n']} | {s['tmin']:.1f}〜{s['tmax']:.1f} | {s['mm']:.2f} | {s['gsd']:.2f} | {s['mm_perm']:.2f} | {s['mm_seed']:.2f} | {seq} | {gp} |")
    allg = [x[0] for x in table]
    big = [x[1] for x in table if x[2]["mm"] >= 2]
    part = [x[1] for x in table if 0 < x[2]["solved"] < x[2]["n"]]
    out.append(f"\n問題の gsd の幾何平均 {math.exp(sum(math.log(g) for g in allg) / len(allg)):.3f}、最大/最小 ≥ 2 の問題 {len(big)}: {', '.join(big)}")
    out.append(f"解けたり解けなかったりする問題 {len(part)}: {', '.join(part)}\n")
print("\n".join(out))
