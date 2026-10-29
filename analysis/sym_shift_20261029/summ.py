"""ab.jsonl の集計: 設定 x 回 x 組 (負けている 21 問・MIPLIB 40 問) の SGM (ずらし 10 秒、解けなければ 60 秒)・解けた数・最適値の食い違い。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[1] / "scripts"))
import run_miplib_benchmark as rb
loss = Path(r"C:\Users\enomo\Desktop\2026\python\ENOMOTO-Solver\.tmpx\loss_names.txt").read_text().split()
sets = {"loss21": loss, "miplib40": rb.PROBLEMS}
R = defaultdict(dict)
for l in open(here / "ab.jsonl", encoding="utf-8"):
    r = json.loads(l); R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
def t(res): return res.get("time", 60) if res.get("status") == "optimal" else 60.0
cfgs = ["new", "-shift", "-sym2"]
for sn, ps in sets.items():
    for rep in (0, 1):
        row = []
        for c in cfgs:
            xs = [t(R[(p, rep)][c]) for p in ps if c in R[(p, rep)]]
            if not xs: continue
            sgm = math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
            row.append(f"{c}: {sgm:6.2f}s ({sum(x < 60 for x in xs)}/{len(xs)})")
        print(f"{sn:9s} rep{rep}  " + "   ".join(row))
print("\n食い違い (どれかが optimal で目的値が違う):")
for (p, rep), d in sorted(R.items()):
    opts = {c: v["objective"] for c, v in d.items() if v.get("status") == "optimal"}
    if opts and max(opts.values()) - min(opts.values()) > 1e-6 * max(1, abs(max(opts.values()))):
        print(" ", p, rep, opts)
    for c, v in d.items():
        if v.get("status") not in ("optimal", "time_limit"): print("  status", p, rep, c, v.get("status"), v.get("error", "")[:100])
print("\n問題ごと (2 回の平均時間, new / -shift / -sym2):")
for p in loss + [x for x in rb.PROBLEMS if x not in loss]:
    v = {c: [t(R[(p, r)][c]) for r in (0, 1) if c in R[(p, r)]] for c in cfgs}
    if all(v.values()):
        m = {c: sum(x) / len(x) for c, x in v.items()}
        flag = " <<" if max(m["-shift"], m["-sym2"]) > 1.3 * m["new"] + 3 else (" >>" if min(m["-shift"], m["-sym2"]) < m["new"] / 1.3 - 3 else "")
        print(f"  {p:22s} " + " / ".join(f"{m[c]:5.1f}" for c in cfgs) + flag)
