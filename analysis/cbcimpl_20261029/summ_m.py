"""Mittelmann の MIP (186 問、1 回) の集計 (abm_s*.jsonl)。分担 18 は止まったので abm_s18b.jsonl (やり直し) だけを使う。
分担 7 はやり直し (abm_s7b.jsonl) の方を優先し、なければ元の abm_s7.jsonl。6 設定がそろった問題で base と比べる"""
import json, math
from pathlib import Path
HERE = Path(__file__).resolve().parent
names = ["base", "tab_nnz", "sbt_abpr", "sbt_full_ps", "tab_abpr", "tab_fullps"]
def load(fn):
    R = {}
    if fn.exists():
        for l in open(fn, encoding="utf-8"):
            r = json.loads(l)
            R.setdefault(r["problem"], {})[r["cfg"]] = r["result"]
    return R
R = {}
for k in range(20):
    if k == 18:
        part = load(HERE / "abm_s18b.jsonl")
    elif k == 7:
        b = load(HERE / "abm_s7b.jsonl")
        part = b if b else load(HERE / "abm_s7.jsonl")
    else:
        part = load(HERE / f"abm_s{k}.jsonl")
    R.update({p: d for p, d in part.items() if all(c in d for c in names)})
def t(x): return min(x.get("time", 60), 60.0) if x.get("status") == "optimal" else 60.0
def ok(x): return x.get("status") == "optimal" and x.get("time", 99) < 60
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
ps = sorted(R)
print(f"6 設定がそろった問題: {len(ps)} / 186")
b = sgm([t(R[p]["base"]) for p in ps])
for c in names:
    xs = [t(R[p][c]) for p in ps]
    g = [p for p in ps if ok(R[p][c]) and not ok(R[p]["base"])]
    l = [p for p in ps if not ok(R[p][c]) and ok(R[p]["base"])]
    print(f"  {c:12s} SGM {sgm(xs):6.2f} {sgm(xs) / b - 1:+6.1%}  解けた {sum(ok(R[p][c]) for p in ps):3d}  (+{len(g)} {g}) (-{len(l)} {l})")
print("\n問題ごと (base と 20% 以上かつ 5 秒以上違うもの、秒):")
for p in ps:
    bt = t(R[p]["base"])
    d = [f"{c}={t(R[p][c]):.0f}" for c in names[1:] if abs(t(R[p][c]) - bt) > max(5, 0.2 * bt)]
    if d:
        print(f"  {p:28s} base={bt:.0f}  " + " ".join(d))
solu = {}
for l in open(HERE.parents[1] / "scripts" / "miplib2017-v31.solu"):
    s = l.split()
    if len(s) >= 3 and s[0] in ("=opt=", "=best="):
        solu[s[1]] = float(s[2])
bad = [(p, c, v.get("objective"), solu.get(p)) for p, d in R.items() for c, v in d.items()
       if (v.get("status") == "optimal" and p in solu and abs(v["objective"] - solu[p]) > 1e-6 * max(1, abs(solu[p])) + 1e-6) or v.get("status") in ("crash", "error")]
print("\n最適値の食い違い・異常終了:", bad)
