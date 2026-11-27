"""今回の比と前回の比 (picked.json の出どころ) を並べる: python compare.py [res_s*.jsonl ...] → 標準出力に Markdown の表。
- 比は当方 / 相手の時間 (shift 1 秒、解けない回は上限で数える)。今回は種 1 の 1 回。
- 組: ours_t1 / scip (前回は vsscip の def_t1 / scip)、ours_t4 / scip (前回は def / scip)、ours_t1 / highs_t1 (前回なし)、ours_t4 / highs_t4 (前回は vshighs の def / highs)。
- 「今も 2 倍以上負け」: どれかの組で今回の比が 2 以上か、相手だけ解けた。前回の倍率と並べ、縮んだ (今回 < 前回 / 1.25)・広がった (今回 > 前回 × 1.25)・同じくらいを書く。
- 当方の手がかり: 根の下界 (root_bound)、最初の解の時間と値、ノード数。相手の下界・ノード数も並べる (負けの型の見立て用)。"""
import collections, glob, json, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
files = sys.argv[1:] or [str(D / "res_s*.jsonl")]
R = {}
for f in files:
    for g in glob.glob(f):
        for l in open(g, encoding="utf-8"):
            if l.strip():
                r = json.loads(l); R[(r["problem"], r["cfg"])] = r
PICK = json.loads((D / "picked.json").read_text(encoding="utf-8"))
def st(r):
    s = str(r["result"].get("status") or "").lower()
    return {"time limit reached": "time_limit"}.get(s, s)
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) and r["result"].get("time") is not None else r["tl"]
def ratio(a, b): return (t(a) + 1) / (t(b) + 1)
PAIRS = [("ours_t1", "scip", "def_t1/scip"), ("ours_t4", "scip", "def/scip"), ("ours_t1", "highs_t1", None), ("ours_t4", "highs_t4", "def/highs")]
def prev(p, key):
    if key is None: return None
    v = [x["ratio"] for x in PICK.get(p, []) if x["src"].endswith(key) and x["ratio"] is not None]
    return max(v) if v else None
probs = sorted({p for p, c in R})
print(f"問題 {len(probs)}、行 {len(R)}\n")
print("| 問題 | " + " | ".join(f"{a}/{b} (前回)" for a, b, _ in PAIRS) + " | 動き | 当方 t1: 根の下界・最初の解 (秒)・ノード | scip: 下界・ノード | highs_t4: 下界・ノード |")
print("|---|" + "---|" * (len(PAIRS) + 4))
lose = []
for p in probs:
    cells, worst, moves = [], 0.0, []
    for a, b, key in PAIRS:
        if (p, a) not in R or (p, b) not in R:
            cells.append("—"); continue
        ra, rb = R[(p, a)], R[(p, b)]
        x = ratio(ra, rb); pv = prev(p, key)
        only = solved(rb) and not solved(ra)
        if not (solved(ra) or solved(rb)):
            cells.append("両方解けず" + (f" ({pv})" if pv else "")); continue
        worst = max(worst, x if not only else max(x, 2.0))
        tag = " 相手だけ" if only else ""
        cells.append(f"{x:.2f}{tag}" + (f" ({pv})" if pv else ""))
        if pv:
            moves.append("縮んだ" if x < pv / 1.25 else ("広がった" if x > pv * 1.25 else "同じくらい"))
    if worst >= 2.0:
        lose.append(p)
        o = R.get((p, "ours_t1")); s = R.get((p, "scip")); h = R.get((p, "highs_t4"))
        def g(r, k):
            v = r["result"].get(k) if r else None
            return f"{v:.6g}" if isinstance(v, (int, float)) and v is not None and math.isfinite(v) else "—"
        oi = f"{g(o, 'root_bound')}・{g(o, 'first_solution_time')}・{g(o, 'nodes')}" if o else "—"
        print(f"| {p} | " + " | ".join(cells) + f" | {'・'.join(sorted(set(moves))) or '—'} | {oi} | {g(s, 'bound')}・{g(s, 'nodes')} | {g(h, 'bound')}・{g(h, 'nodes')} |")
print(f"\n今も 2 倍以上負け (どれかの組): {len(lose)} 問: {' '.join(lose)}")
