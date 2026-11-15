"""600 秒の解き比べ (heavy_*.jsonl) を集計する (analysis/foldpre_20261112.md)。問題・種ごとに 6 設定の結果を並べ、
解けた回・時間 (時間切れは 600)・ギャップ (上界と下界、解なしは 1) の幾何平均の比を設定ごとに出す。"""
import glob, json, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for fn in sorted(glob.glob(str(D / "heavy_*.jsonl"))):
    for l in open(fn):
        r = json.loads(l)
        R[(r["problem"], r["seed"], r["cfg"])] = (r["result"], Path(fn).stem)
cf = ["nofold", "def", "fP", "fQ", "fB", "fBQ"]
def gap(x):
    o, b = x.get("objective"), x.get("bound")
    if o is None or b is None:
        return 1.0
    return min(1.0, abs(o - b) / max(abs(o), abs(b), 1e-9))
def cell(x):
    if x is None:
        return "-"
    if x.get("status") == "optimal":
        return f"最適 {x['time']:.0f}s"
    if x.get("objective") is None:
        return f"解なし / {x.get('bound', float('nan')):.6g}"
    return f"{x['objective']:.6g} / {x.get('bound', float('nan')):.6g}"
keys = sorted({(p, s) for p, s, _ in R})
print("問題 | 種 | 分担 | " + " | ".join(cf))
for p, s in keys:
    sh = next((R[(p, s, c)][1] for c in cf if (p, s, c) in R), "")
    print(p, "|", s, "|", sh.replace("heavy_", ""), "|", " | ".join(cell(R.get((p, s, c), (None,))[0]) for c in cf))
full = [(p, s) for p, s in keys if all((p, s, c) in R for c in cf)]
print(f"\n6 設定とも揃った組 {len(full)}")
for c in cf:
    solved = sum(R[(p, s, c)][0].get("status") == "optimal" for p, s in full)
    sgm_t = math.exp(sum(math.log(10 + (R[(p, s, c)][0].get("time", 600) if R[(p, s, c)][0].get("status") == "optimal" else 600)) for p, s in full) / max(1, len(full))) - 10
    sgm_g = math.exp(sum(math.log(1e-4 + gap(R[(p, s, c)][0])) for p, s in full) / max(1, len(full)))
    print(f"{c}: 解けた {solved}、時間の SGM(10) {sgm_t:.1f}、ギャップの幾何平均 {sgm_g:.4f}")
