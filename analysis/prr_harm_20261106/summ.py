"""bench58.jsonl の集計: 回ごと・全回の SGM (時間 + 10 秒、解けなければ 60 秒) と解けた数、片方だけ解けた回、時間の差が大きい問題、最適値の食い違い。"""
import json, math, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = collections.defaultdict(dict)
for l in open(D / "bench58.jsonl"):
    d = json.loads(l); R[(d["problem"], d["rep"])][d["cfg"]] = d["result"]
def t(r): return r["time"] if r.get("status") == "optimal" else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
keys = [k for k, v in R.items() if "def" in v and "prr" in v]
for rep in sorted({k[1] for k in keys}) + [None]:
    ks = [k for k in keys if rep is None or k[1] == rep]
    a = [t(R[k]["def"]) for k in ks]; b = [t(R[k]["prr"]) for k in ks]
    na = sum(R[k]["def"].get("status") == "optimal" for k in ks); nb = sum(R[k]["prr"].get("status") == "optimal" for k in ks)
    print(f"rep {rep if rep is not None else '全回'}: {len(ks)} 組, def {sgm(a):.2f} ({na}), prr {sgm(b):.2f} ({nb}), 比 {100*((sgm(b)+10)/(sgm(a)+10)-1):+.1f}%")
print("片方だけ解けた回:")
for k in sorted(keys):
    a, b = R[k]["def"], R[k]["prr"]
    if (a.get("status") == "optimal") != (b.get("status") == "optimal"):
        print(" ", k, "def", a.get("status"), round(t(a), 1), "prr", b.get("status"), round(t(b), 1))
print("時間の比 (t+10) が 1.3 倍を超える回:")
for k in sorted(keys):
    a, b = t(R[k]["def"]), t(R[k]["prr"])
    q = (b + 10) / (a + 10)
    if q > 1.3 or q < 1 / 1.3:
        print(f"  {k} def {a:.1f} prr {b:.1f} 比 {q:.2f}")
print("最適値の食い違い:")
for k in sorted(keys):
    a, b = R[k]["def"], R[k]["prr"]
    if a.get("status") == "optimal" and b.get("status") == "optimal" and abs(a["objective"] - b["objective"]) > 1e-6 * (1 + abs(a["objective"])):
        print(" ", k, a["objective"], b["objective"])
print("crash:", [k for k in keys if any(R[k][c].get("status", "").startswith("crash") for c in ("def", "prr"))])
