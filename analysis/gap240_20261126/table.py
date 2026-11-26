"""他のソルバーとの差の表 (2026-11-26、親の依頼): python table.py > table.txt (gap.json も書く)
当方は combo2 第 2 段 (analysis/combo2_20261125/s2、c8e468e25、種 1・600 秒) の def_t1・def_t4・c2c_t1・c2c_t4。
SCIP は analysis/vsscip_20261117 の M (SCIP 10.0.2・逐次・種 1・600 秒、Mittelmann 186 問)。
HiGHS は analysis/vshighs_20261117 の M・A (HiGHS 1.15.1・4 スレッド・600 秒。M の 48 問は種 1・2、A の 138 問は種 1。ここでは種 1 を使う)。
240 問のうち残りの 54 問は SCIP・HiGHS の 600 秒の結果がない (表では「—」)。
行の重複は combo2 の summ.py と同じ決まり (crash・取得失敗でない行を優先し、その中では後の行)。"""
import glob, json, math
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parent
TL, SH = 600.0, 10.0


def load(pat, keep=lambda r: True):
    R = {}
    for g in sorted(glob.glob(str(A / pat))):
        for l in open(g, encoding="utf-8"):
            if not l.strip():
                continue
            r = json.loads(l)
            if not keep(r):
                continue
            key = (r["problem"], r["cfg"], r.get("seed", 1))
            if key not in R or str(r["result"].get("status") or "").lower() not in ("crash", "fetch_error"):
                R[key] = r
    return R


def norm(s):
    s = str(s or "").lower()
    return {"time limit reached": "time_limit"}.get(s, s)


def st(r): return norm(r["result"].get("status")) if r else "—"
def solved(r): return r is not None and st(r) in ("optimal", "infeasible")
def tm(r): return min(r["result"].get("time") or TL, TL) if solved(r) else TL


PJ = json.loads((A / "combo2_20261125/s2/problems.json").read_text())
M = set(PJ["M"])
OURS = load("combo2_20261125/s2/res_*.jsonl")
SCIP = load("vsscip_20261117/res_M_*.jsonl", lambda r: r["cfg"] == "scip" and r["seed"] == 1)
HIGHS = load("vshighs_20261117/res_[MA]_*.jsonl", lambda r: r["cfg"] == "highs" and r["seed"] == 1)
CFG = ["def_t1", "def_t4", "c2c_t1", "c2c_t4"]

rows = []
for p in sorted(PJ["all"]):
    o = {c: OURS.get((p, c, 1)) for c in CFG}
    s = SCIP.get((p, "scip", 1)); h = HIGHS.get((p, "highs", 1))
    d = {"problem": p, "set": "M" if p in M else "rest"}
    for c in CFG:
        r = o[c]
        d[c] = {"status": st(r), "time": round(tm(r), 1) if r else None, "nodes": r["result"].get("nodes") if r else None,
                "obj": r["result"].get("objective") if r else None, "bound": r["result"].get("bound") if r else None,
                "root_bound": r["result"].get("root_bound") if r else None,
                "first_sol": r["result"].get("first_solution_time") if r else None}
    for name, r in (("scip", s), ("highs", h)):
        d[name] = None if r is None else {"status": st(r), "time": round(tm(r), 1), "nodes": r["result"].get("nodes"),
                                          "obj": r["result"].get("objective"), "bound": r["result"].get("bound")}
    rows.append(d)


def ok(x): return x is not None and x["status"] in ("optimal", "infeasible")
def T(x): return x["time"] if ok(x) else TL


def opp_best(d):
    c = [(T(d[k]), k) for k in ("scip", "highs") if ok(d[k])]
    return min(c) if c else None


# 組は公平な対にそろえる: SCIP (逐次) は当方の 1 スレッド (def_t1)、HiGHS (4 スレッド) は当方の 4 スレッド (def_t4) と比べる。
#  a1: SCIP は解け、def_t1 は解けない / a4: HiGHS は解け、def_t4 は解けない
#  b1: 両方解け、def_t1 が SCIP の 3 倍以上 ((当方+1)/(相手+1) >= 3) / b4: 同じく def_t4 と HiGHS
#  c: Mittelmann の問題で、SCIP・HiGHS のどちらも解けず、当方 (既定・c2c のどれか) は解ける
def R(a, b): return (T(a) + 1) / (T(b) + 1)
cls_a, cls_b, cls_c = [], [], []
for d in rows:
    tags = []
    if ok(d["scip"]):
        if not ok(d["def_t1"]): tags.append("a1")
        elif R(d["def_t1"], d["scip"]) >= 3: tags.append("b1")
    if ok(d["highs"]):
        if not ok(d["def_t4"]): tags.append("a4")
        elif R(d["def_t4"], d["highs"]) >= 3: tags.append("b4")
    if not tags and d["set"] == "M" and not ok(d["scip"]) and not ok(d["highs"]) and any(ok(d[c]) for c in CFG):
        tags.append("c")
    d["cls"] = ",".join(tags)
    if any(t[0] == "a" for t in tags): cls_a.append(d)
    elif any(t[0] == "b" for t in tags): cls_b.append(d)
    elif tags: cls_c.append(d)

# MIPLIB 2017 の札 (analysis/orbpp_20261121/collection.json、miplib.zib.de の一覧の写し)
COL = {c["name"]: c for c in json.loads((A / "orbpp_20261121/collection.json").read_text())}
TAGSKIP = {"benchmark", "benchmark_suitable", "feasibility", "infeasible", "decomposition", "binary", "mixed_binary", "integer"}
def mtags(p):
    c = COL.get(p)
    if not c: return ""
    return " ".join(t for t in c["tags"] if t not in TAGSKIP)
def size(p):
    c = COL.get(p)
    return f"{c['vars']}列 (b{c['bins']} i{c['ints']} c{c['conts']}) {c['cons']}行" if c else ""

def f(x):
    if x is None: return "—"
    s = x["status"]
    if s == "optimal": return f"{x['time']:.0f}"
    if s == "infeasible": return f"{x['time']:.0f}i"
    if s == "time_limit": return "TL"
    return s


def sgm(ts): return math.exp(sum(math.log(t + SH) for t in ts) / len(ts)) - SH


print("== 1. 問題ごとの表 (数は秒、TL = 600 秒の時間切れ、i = 実行不能を示して終わり、— = 結果なし)")
print("当方: combo2 第 2 段 (c8e468e25、種 1)。SCIP 10.0.2 は逐次、HiGHS 1.15.1 は 4 スレッド (どちらも 600 秒・種 1)。分類 a・b・c は 2 節。\n")
print("| 問題 | 集合 | 既定 t1 | 既定 t4 | c2c t1 | c2c t4 | SCIP (t1) | HiGHS (t4) | 分類 |")
print("|---|---|---:|---:|---:|---:|---:|---:|---|")
for d in rows:
    print(f"| {d['problem']} | {d['set']} | {f(d['def_t1'])} | {f(d['def_t4'])} | {f(d['c2c_t1'])} | {f(d['c2c_t4'])} | {f(d['scip'])} | {f(d['highs'])} | {d.get('cls', '')} |")

Mrows = [d for d in rows if d["set"] == "M"]
print("\n== 解けた数と SGM (Mittelmann 186 問、shift 10、時間切れは 600 秒)")
for k in CFG + ["scip", "highs"]:
    ts = [T(d[k]) for d in Mrows]
    print(f"  {k:7s}: 解けた {sum(ok(d[k]) for d in Mrows):3d}  SGM {sgm(ts):6.1f} 秒")
vb = [min([T(d[k]) for k in ("scip", "highs")]) for d in Mrows]
print(f"  相手の良い方 (SCIP・HiGHS の速い方): 解けた {sum(t < TL for t in vb)}  SGM {sgm(vb):.1f} 秒")

json.dump(rows, open(D / "gap.json", "w"), ensure_ascii=False, indent=0)
print(f"\n(a) {len(cls_a)} 問、(b) {len(cls_b)} 問、(c) {len(cls_c)} 問")

N = len(Mrows)
def eff(e, o):
    """当方の時間 e を相手の時間 o に置き換えたときの SGM (186 問、shift 10) の比"""
    return math.exp((math.log(min(e, o) + SH) - math.log(e + SH)) / N)
print("\n== 2. (a)・(b) の一覧 (公平な対: SCIP 逐次 ↔ 当方 t1、HiGHS 4 スレッド ↔ 当方 t4)")
print("SGM の効き = その問題で当方の時間を相手の時間まで縮めたとき、186 問の SGM (shift 10) が何倍になるか。時間切れは 600 秒で数えるので、解けない問題の効きは控えめ (下限)。\n")
print("| 問題 | 分類 | 既定 t1 | 既定 t4 | c2c t1 | c2c t4 | SCIP | HiGHS | 効き t1 (対 SCIP) | 効き t4 (対 HiGHS) | 大きさ | MIPLIB の札 |")
print("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|")
for d in cls_a + cls_b:
    e1 = eff(T(d["def_t1"]), T(d["scip"])) if ok(d["scip"]) else 1.0
    e4 = eff(T(d["def_t4"]), T(d["highs"])) if ok(d["highs"]) else 1.0
    d["eff1"], d["eff4"] = e1, e4
    print(f"| {d['problem']} | {d['cls']} | {f(d['def_t1'])} | {f(d['def_t4'])} | {f(d['c2c_t1'])} | {f(d['c2c_t4'])} | {f(d['scip'])} | {f(d['highs'])} | {e1:.4f} | {e4:.4f} | {size(d['problem'])} | {mtags(d['problem'])} |")
print(f"\n全部を相手の時間まで縮めたときの SGM の比: t1 {math.prod(d['eff1'] for d in cls_a + cls_b):.3f}、t4 {math.prod(d['eff4'] for d in cls_a + cls_b):.3f}")
print("\n== (c) 当方だけ解ける問題 (参考、Mittelmann で SCIP・HiGHS とも 600 秒で解けない)\n")
print("| 問題 | 既定 t1 | 既定 t4 | c2c t1 | c2c t4 | MIPLIB の札 |")
print("|---|---:|---:|---:|---:|---|")
for d in cls_c:
    print(f"| {d['problem']} | {f(d['def_t1'])} | {f(d['def_t4'])} | {f(d['c2c_t1'])} | {f(d['c2c_t4'])} | {mtags(d['problem'])} |")
# 3 倍に届かないが 2 倍以上遅い問題 (参考)
print("\n== 参考: 両方解けて 2〜3 倍遅い問題 (公平な対)\n")
for d in Mrows:
    for opp, me in (("scip", "def_t1"), ("highs", "def_t4")):
        if ok(d[opp]) and ok(d[me]) and 2 <= R(d[me], d[opp]) < 3:
            print(f"  {d['problem']} {me} {T(d[me]):.0f} 秒 / {opp} {T(d[opp]):.0f} 秒 (比 {R(d[me], d[opp]):.2f}) {mtags(d['problem'])}")
