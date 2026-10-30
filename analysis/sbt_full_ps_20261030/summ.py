"""ab*.jsonl と 6 段目 (analysis/cbcimpl_20261029/ab6_s*.jsonl) の noswot・timtab1 を設定ごとに並べ、ログのカウンタを集計する。"""
import glob, gzip, json, math, re, sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROBS = ["noswot", "timtab1"]
rows = defaultdict(list)  # (prob, cfg) -> [(time or None, nodes, rep, src)]
for f in sorted(glob.glob(str(HERE / "ab*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        x = r["result"]
        t = x["time"] if x.get("status") == "optimal" else None
        rows[(r["problem"], r["cfg"])].append((t, x.get("nodes"), r["rep"], Path(f).name, x.get("bound"), x.get("objective")))
ab6 = defaultdict(list)
for f in sorted(glob.glob(str(HERE.parent / "cbcimpl_20261029/ab6_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        if r["problem"] in PROBS:
            x = r["result"]
            cfg = "base" if r["cfg"] == "base2" else r["cfg"]
            ab6[(r["problem"], cfg)].append(x["time"] if x.get("status") == "optimal" else None)

CO = ["base", "sbt_abpr", "sbt_abpr_ps", "sbt_full_ps", "full_no_ps", "full_no_rc", "full_no_prc"]

def sgm(ts, cap=60.0, s=10.0):
    return math.exp(sum(math.log(min(t if t is not None else cap, cap) + s) for t in ts) / len(ts)) - s

def fmt(ts):
    return " ".join("TO" if t is None else f"{t:.1f}" for t in ts)

print("| 問題 | 設定 | この計測 (秒) | 解けた | SGM(10) | 60 秒での下界の平均 (時間切れの回) | 6 段目 (秒) |")
print("|---|---|---|---|---|---|---|")
for p in PROBS:
    for c in sorted({k[1] for k in rows if k[0] == p}, key=lambda c: CO.index(c) if c in CO else 99):
        ts = [r[0] for r in sorted(rows[(p, c)], key=lambda r: (r[3], r[2]))]
        lbs = [r[4] for r in rows[(p, c)] if r[0] is None and r[4] is not None]
        lb = f"{sum(lbs) / len(lbs):.6g} ({len(lbs)})" if lbs else "-"
        print(f"| {p} | {c} | {fmt(ts)} | {sum(t is not None for t in ts)}/{len(ts)} | {sgm(ts):.1f} | {lb} | {fmt(ab6.get((p, c), []))} |")

# ログのカウンタ
pat = {
    "sel": r"branching: (\d+) selections, (\d+) without",
    "sblps": r"sb LPs (\d+) iters (\d+)",
    "diag": r"poly diag(?: \(all tree threads\))?: selections with polys (\d+), candidates with polys (\d+) \(raised (\d+), not raised (\d+)\), top changed (\d+); branches (\d+) \(on raised (\d+), raised without sb (\d+), on cols with polys (\d+)\); mean raise x([\d.]+) over (\d+) sides, mean poly/pc x([\d.]+) over (\d+) sides",
    "inh": r"inherited bounds applied (\d+), ancestor convexity tightenings (\d+), reduced cost updates at (\d+)",
    "tight": r"tightened (\d+) general integer bounds by convexity",
    "rc": r"lines from child reduced costs (\d+)",
    "workers": r"worker nodes (\d+)",
}
print()
print("| 問題 | 設定 | 回 | 秒 | ノード | 選択 (主) | 強分岐 LP | 折れ線のある選択 | 引き上げた候補 / 折れ線のある候補 | 1 位が変わった | 引き上げた列で分枝 / 分枝 | 折れ線/擬似費用 (幾何平均) | 祖先の締め付け | 凸性の締め付け | 被約費用の直線 | 診断の範囲 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for p in PROBS:
    for c in CO:
        for (t, nodes, rep, src, *_rest) in sorted(rows.get((p, c), []), key=lambda r: (r[3], r[2])):
            lf = HERE / "logs" / f"{p}_{c}_{rep}.log.gz"
            if not lf.exists():
                continue
            s = gzip.open(lf, "rt").read()
            g = {k: (re.findall(v, s) or [None])[-1] for k, v in pat.items()}
            d = g["diag"]
            print(f"| {p} | {c} | {rep} | {'TO' if t is None else f'{t:.1f}'} | {nodes} | {g['sel'][0] if g['sel'] else '-'} | {g['sblps'][0] if g['sblps'] else '-'} | "
                  + (f"{d[0]} | {d[2]}/{d[1]} | {d[4]} | {d[6]}/{d[5]} | {float(d[11]):.2f} | " if d else "- | - | - | - | - | ")
                  + f"{g['inh'][1] if g['inh'] else '-'} | {g['tight'] or '-'} | {g['rc'] or '-'} | {'全スレッド' if 'all tree threads' in s else '主のみ'} |")
