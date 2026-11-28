"""PRESIZE240: res.jsonl (当方・HiGHS) と scip.jsonl (SCIP) から、前処理の後の大きさの比の表を作る。
速さは analysis/gap240_20261126/gap.json (当方 def_t1・def_t4、SCIP 逐次、HiGHS 4 スレッド、600 秒) から取る。
出力: table.txt (全問の表・比の分布・1.2 倍以上の一覧)、big.json (差の大きい問題)。"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent
R = {json.loads(l)["problem"]: json.loads(l) for l in open(D / "res.jsonl")}
S = {json.loads(l)["problem"]: json.loads(l)["scip"] for l in open(D / "scip.jsonl")}
G = {g["problem"]: g for g in json.load(open(D.parent / "gap240_20261126/gap.json"))}
def rat(a, b): return a / b if b else (math.inf if a else 1.0)
def tm(g, k):
    x = (g or {}).get(k)
    return None if not x else (x["time"] if x["status"] in ("optimal", "infeasible") else 600.0)
rows = []
for p, r in R.items():
    o, h, s = r["ours"], r["highs"], S.get(p, {})
    rec = {"problem": p, "orig": r["orig"], "ours": {k: o.get(k) for k in ("rows", "cols", "nnz", "t")}, "highs": {k: h.get(k) for k in ("rows", "cols", "nnz", "t")},
           "scip": {"rows": s.get("rows", 0 if s.get("status") == "optimal" else None), "cols": s.get("vars"), "nnz": s.get("nnz", 0 if s.get("status") == "optimal" else None), "t": s.get("t")}}
    g = G.get(p)
    rec["time"] = {k: tm(g, k) for k in ("def_t1", "def_t4", "scip", "highs")}
    rec["cls"] = (g or {}).get("cls")
    if o.get("rows") is None:
        fin = (o.get("final") or {}).get("status")
        rec["note"] = "根の LP の前に解けた" if fin == "optimal" else "600 秒で根の LP の大きさが出ない"
    else:
        rec["rH"], rec["cH"], rec["nH"] = (round(rat(o[k], h[k]), 3) for k in ("rows", "cols", "nnz"))
        if rec["scip"]["rows"] is not None:
            rec["rS"], rec["cS"], rec["nS"] = (round(rat(o[k], rec["scip"][k]), 3) for k in ("rows", "cols", "nnz"))
    rows.append(rec)
rows.sort(key=lambda r: -max(r.get("rH", 0), r.get("cH", 0), r.get("nH", 0)))
def f(x): return "—" if x is None else ("∞" if x == math.inf else f"{x:.2f}")
def t(x): return "—" if x is None else f"{x:.0f}"
out = ["| 問題 | 元 行×列 | 当方 行×列 (非零) | HiGHS 行×列 (非零) | SCIP 行×列 (非零) | 当方/HiGHS 行・列・非零 | 当方/SCIP 行・列・非零 | 秒 当方1T・SCIP・当方4T・HiGHS | 型 |",
       "|---|---|---|---|---|---|---|---|---|"]
for r in rows:
    o, h, s, tt = r["ours"], r["highs"], r["scip"], r["time"]
    if "note" in r:
        oo = r["note"]
    else:
        oo = f"{o['rows']}×{o['cols']} ({o['nnz']})"
    out.append(f"| {r['problem']} | {r['orig']['rows']}×{r['orig']['cols']} | {oo} | {h['rows']}×{h['cols']} ({h['nnz']}) | {s['rows']}×{s['cols']} ({s['nnz']}) | "
               f"{f(r.get('rH'))}・{f(r.get('cH'))}・{f(r.get('nH'))} | {f(r.get('rS'))}・{f(r.get('cS'))}・{f(r.get('nS'))} | "
               f"{t(tt['def_t1'])}・{t(tt['scip'])}・{t(tt['def_t4'])}・{t(tt['highs'])} | {r['cls'] or ''} |")
ok = [r for r in rows if "rH" in r]
def gm(v): v = [x for x in v if 0 < x < math.inf]; return math.exp(sum(map(math.log, v)) / len(v))
head = [f"240 問のうち比べられた {len(ok)} 問。当方/HiGHS の幾何平均: 行 {gm([r['rH'] for r in ok]):.3f}・列 {gm([r['cH'] for r in ok]):.3f}・非零 {gm([r['nH'] for r in ok]):.3f}",
        f"当方/SCIP の幾何平均 ({sum('rS' in r for r in ok)} 問): 行 {gm([r['rS'] for r in ok if 'rS' in r]):.3f}・列 {gm([r['cS'] for r in ok if 'cS' in r]):.3f}・非零 {gm([r['nS'] for r in ok if 'nS' in r]):.3f}"]
for k, nm in (("rH", "行"), ("cH", "列"), ("nH", "非零")):
    v = [r[k] for r in ok]
    head.append(f"{nm} の比 (当方/HiGHS): <0.9 {sum(x < 0.9 for x in v)}、0.9〜1.1 {sum(0.9 <= x < 1.1 for x in v)}、1.1〜1.2 {sum(1.1 <= x < 1.2 for x in v)}、≥1.2 {sum(x >= 1.2 for x in v)}")
big = [r for r in ok if max(r["rH"], r["cH"], r["nH"]) >= 1.2]
(D / "table.txt").write_text("\n".join(head) + "\n\n" + "\n".join(out) + "\n")
json.dump(big, open(D / "big.json", "w"), ensure_ascii=False, indent=0)
print("\n".join(head)); print("1.2 倍以上", len(big))
