"""負けている問題を拾う: python pick.py → picked.json と表 (標準出力)。
出どころ (どれも当方 / 相手の時間の比、shift 1 秒、解けない回は上限で数え、種ごとの比の幾何平均):
- vsscip_20261120 の M (今の既定 def・def_t1、相手は 11/17 の scip の行。summ_M.txt と同じ組)
- vsscip_20261117 の P (def・def_t1 / scip、種 1〜3)
- vshighs_20261117 の M・A (MA = Mittelmann 186 問) と P (def / highs、4 スレッド)
拾う条件: 比が 1.5 以上、または相手だけが解けた回がある。さらに comp07-2idx (RINS_PAR の M で既定でも時間のかかった問題) を足す。"""
import collections, glob, json, math
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parent
def load(pats):
    out = []
    for pat in pats:
        for f in glob.glob(str(A / pat)):
            out += [json.loads(l) for l in open(f) if l.strip()]
    return out
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
src = []
# (名前, 当方の行, 相手の行, 当方の設定, 相手の設定)
m20 = load(["vsscip_20261120/res_M_s*.jsonl"]); s17M = [r for r in load(["vsscip_20261117/res_M_s*.jsonl"]) if r["cfg"] == "scip"]
s17P = load(["vsscip_20261117/res_P_s*.jsonl"])
vhM = load(["vshighs_20261117/res_M_s*.jsonl", "vshighs_20261117/res_A_s*.jsonl"]); vhP = load(["vshighs_20261117/res_P_s*.jsonl"])
src = [("vsscip1120 M def/scip", m20, s17M, "def", "scip"), ("vsscip1120 M def_t1/scip", m20, s17M, "def_t1", "scip"),
       ("vsscip1117 P def/scip", s17P, s17P, "def", "scip"), ("vsscip1117 P def_t1/scip", s17P, s17P, "def_t1", "scip"),
       ("vshighs1117 MA def/highs", vhM, vhM, "def", "highs"), ("vshighs1117 P def/highs", vhP, vhP, "def", "highs")]
pick = collections.defaultdict(list)
for name, ours, opp, co, cp in src:
    O = {(r["problem"], r["seed"]): r for r in ours if r["cfg"] == co}
    Q = {(r["problem"], r["seed"]): r for r in opp if r["cfg"] == cp}
    lr = collections.defaultdict(list); only = collections.Counter(); tl = {}
    for k in O.keys() & Q.keys():
        a, b = O[k], Q[k]
        tl[k[0]] = a["tl"]
        if solved(b) and not solved(a): only[k[0]] += 1
        if solved(a) or solved(b): lr[k[0]].append(math.log((t(a) + 1) / (t(b) + 1)))
    for p, v in lr.items():
        g = math.exp(sum(v) / len(v))
        if g >= 1.5 or only[p]:
            pick[p].append({"src": name, "ratio": round(g, 2), "opp_only": only[p], "n": len(v), "tl": tl[p]})
pick["comp07-2idx"].append({"src": "RINS_PAR の M (既定でも時間がかかった)", "ratio": None, "opp_only": 0, "n": 0, "tl": 600})
json.dump(dict(sorted(pick.items())), open(D / "picked.json", "w"), indent=1, ensure_ascii=False)
print(len(pick), "問")
for p in sorted(pick, key=lambda q: -max((x["ratio"] or 0) for x in pick[q])):
    print(p, "; ".join(f"{x['src']} {x['ratio']}" + (f" (相手だけ {x['opp_only']}/{x['n']})" if x["opp_only"] else "") for x in pick[p]))
