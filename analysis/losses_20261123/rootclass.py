"""27 問の根の下界の比べと型分け: python rootclass.py → 標準出力に Markdown の表。
根の下界: 当方は res_s*.jsonl の root_bound (根の切除平面ループの後、再スタートがあれば最後の根)、SCIP は limits/nodes = 1、HiGHS は mip_max_nodes = 1 (rootcmp.jsonl)。
型分け: 相手 (SCIP・HiGHS の良い方) の根の下界が当方 (1・4 スレッドの良い方) より、既知の最適値の絶対値 (1 以上) の 0.1% を超えて高ければ「根の下界の差」、
そうでなければ「根の後の差」。実行不能の問題は「根の後の差 (証明)」。どの問題も最小化。"""
import collections, glob, json, math
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for f in glob.glob(str(D / "res_s*.jsonl")):
    for l in open(f):
        if l.strip():
            r = json.loads(l); R[(r["problem"], r["cfg"])] = r
RC = {}
SOL = json.loads((D / "solu.json").read_text())
for l in open(D / "rootcmp.jsonl"):
    r = json.loads(l); res = dict(r["result"])
    # SCIP が根で解き切ると、根の下界は無限大で返る。そのときは根の下界を最適値とみなす
    if not (isinstance(res.get("root"), (int, float)) and abs(res["root"]) < 1e19) and str(res.get("status")).lower() == "optimal":
        res["root"] = (SOL.get(r["problem"]) or {}).get("value")
    RC[(r["problem"], r["solver"])] = res
probs = [p for p in open(D / "compare.txt", encoding="utf-8").read().split("今も 2 倍以上負け (どれかの組): 27 問: ")[1].split()]
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def fin(v): return isinstance(v, (int, float)) and math.isfinite(v) and abs(v) < 1e19
def f(v): return f"{v:.6g}" if fin(v) else "—"
rows = {"root": [], "after": []}
for p in probs:
    sol = SOL.get(p, {}); opt = sol.get("value") if sol.get("kind") in ("opt", "best") else None
    o1, o4 = R[(p, "ours_t1")], R[(p, "ours_t4")]
    ours = [x["result"].get("root_bound") for x in (o1, o4) if fin(x["result"].get("root_bound"))]
    opp = [RC.get((p, s), {}).get("root") for s in ("scip", "highs")]
    opp = [x for x in opp if fin(x)]
    if sol.get("kind") == "inf" or not ours or not opp or opt is None:
        kind = "after"
    else:
        kind = "root" if max(opp) - max(ours) > 1e-3 * max(1.0, abs(opt)) else "after"
    def tn(r):
        res = r["result"]; t = min(res["time"], r["tl"]) if solved(r) else r["tl"]; n = res.get("nodes") or 0
        return t, n
    line = [p, f(opt) if opt is not None else sol.get("kind", "—"), f(o1["result"].get("root_bound")), f(o4["result"].get("root_bound")),
            f(RC.get((p, "scip"), {}).get("root")), f(RC.get((p, "highs"), {}).get("root"))]
    if kind == "after":
        cells = []
        for c in ("ours_t1", "ours_t4", "scip", "highs_t1", "highs_t4"):
            r = R[(p, c)]; t, n = tn(r)
            fs = r["result"].get("first_solution_time")
            cells.append(f"{'TL' if not solved(r) else f'{t:.0f}'}・{n}" + (f"・初 {fs:.1f}" if fin(fs) else ""))
        line += cells
    rows[kind].append(line)
print("### 根の下界の差がある (相手の根の下界が当方より高い)\n")
print("| 問題 | 最適値 | 当方 t1 の根 | 当方 t4 の根 | SCIP の根 | HiGHS の根 |")
print("|---|---:|---:|---:|---:|---:|")
for x in rows["root"]: print("| " + " | ".join(x) + " |")
print(f"\n{len(rows['root'])} 問\n\n### 根の後の差 (根の下界は並ぶか当方が上、または実行不能の証明)\n")
print("時間 (秒、TL は 600 秒の時間切れ)・ノード数・初 = 最初の解の時刻 (当方だけ)。\n")
print("| 問題 | 最適値 | 当方 t1 の根 | 当方 t4 の根 | SCIP の根 | HiGHS の根 | ours_t1 | ours_t4 | scip | highs_t1 | highs_t4 |")
print("|---|---:|---:|---:|---:|---:|---|---|---|---|---|")
for x in rows["after"]: print("| " + " | ".join(x) + " |")
print(f"\n{len(rows['after'])} 問")
