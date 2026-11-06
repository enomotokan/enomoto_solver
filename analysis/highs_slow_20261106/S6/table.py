"""runs.jsonl と前の 600 秒の比べ (mitt600_20261103 の res_s*/res2_s*) から、問題 x 設定ごとの結果の表を出す (S6.md 用)。"""
import json, glob, collections
from pathlib import Path
D = Path(__file__).resolve().parent
P = ["nexp-150-20-8-5", "neos-662469", "trento1", "thor50dday"]
rows = collections.defaultdict(list)
for f in sorted(glob.glob(str(D.parents[1] / "mitt600_20261103/res2_s*.jsonl"))) + sorted(glob.glob(str(D.parents[1] / "mitt600_20261103/res_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        if r["problem"] in P and r["solver"] in ("ours", "highs"):
            x = r["result"]; o, b = x.get("objective"), x.get("bound")
            g = abs(o - b) / max(abs(o), abs(b), 1e-9) if o is not None and b is not None else None
            name = "def(res2)" if r["solver"] == "ours" else "highs(res)"
            if "res2" not in f and r["solver"] == "ours": name = "def(res)"
            rows[(r["problem"], name)].append((o, b, g, x.get("root_bound"), x.get("first_solution_time"), "-"))
for l in open(D / "runs.jsonl"):
    r = json.loads(l); x = r["result"]
    rows[(r["problem"], r["name"])].append((x.get("objective"), x.get("bound"), r["gap"], x.get("root_bound"), x.get("first_solution_time"), r["seed"]))
f = lambda v, d=6: "-" if v is None else f"{v:.{d}g}"
for p in P:
    print(f"\n#### {p}\n\n| 設定 | 種 | 暫定解 | 下界 | ギャップ | 根の下界 |\n|---|---|---|---|---|---|")
    for (pp, n), v in rows.items():
        if pp != p: continue
        for o, b, g, rb, fs, s in v:
            print(f"| {n} | {s} | {f(o, 8)} | {f(b, 8)} | {'-' if g is None else f'{100*g:.3g}%'} | {f(rb, 8)} |")
