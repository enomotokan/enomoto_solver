"""確かめの集計: python check.py > check.txt。def_t1 と c1119_t1 (1 スレッド、同じになるはず。時間切れの回は壁時計で節点数が揺れる)、
def と cand1119 の回・rev と def (= cand1118) の回 (cand1119_20261117 の res_P、種 1、4 スレッド、揺れの範囲で近いはず) を比べる。
状態・目的値・時間・節点数。4 スレッドなので節点数・時間は揺れうる (TICKS・DETHASH で決定的な部分は一致する見込み)。"""
import json, glob, math
from pathlib import Path
D = Path(__file__).resolve().parent
R = {(r["problem"], r["cfg"]): r["result"] for f in glob.glob(str(D / "res_P_s*.jsonl")) for l in open(f) if l.strip() for r in [json.loads(l)]}
O = {r["problem"]: r["result"] for f in glob.glob(str(D.parent / "cand1119_20261117/res_P_s*.jsonl")) for l in open(f) for r in [json.loads(l)] if r["cfg"] == "def" and r["seed"] == 1}
N = {r["problem"]: r["result"] for f in glob.glob(str(D.parent / "cand1119_20261117/res_P_s*.jsonl")) for l in open(f) for r in [json.loads(l)] if r["cfg"] == "cand1119" and r["seed"] == 1}
probs = sorted({p for p, c in R})
def st(x): return str(x.get("status") or "").lower()
def t(x): return min(x.get("time") or 60, 60) if st(x) == "optimal" else 60
def sgm(v): return math.exp(sum(math.log(a + 1) for a in v) / len(v)) - 1 if v else float("nan")
print("行数", len(R), "問題", len(probs))
for a, b in (("def_t1", "c1119_t1"), ("def", "cand1119 (前の回)"), ("rev", "cand1118 (前の回の def)")):
    pairs = [(p, R.get((p, a)), R.get((p, b)) if b == "c1119_t1" else (N.get(p) if b.startswith("cand1119") else O.get(p))) for p in probs]
    pairs = [(p, x, y) for p, x, y in pairs if x and y]
    ss = sum(st(x) == st(y) for p, x, y in pairs)
    nn = sum(x.get("nodes") == y.get("nodes") for p, x, y in pairs)
    bad = [(p, x.get("objective"), y.get("objective")) for p, x, y in pairs if st(x) == st(y) == "optimal" and abs((x.get("objective") or 0) - (y.get("objective") or 0)) > 1e-6 * max(1, abs(y.get("objective") or 0))]
    print(f"\n== {a} 対 {b}: {len(pairs)} 問、解けた {sum(st(x) == 'optimal' for p, x, y in pairs)} / {sum(st(y) == 'optimal' for p, x, y in pairs)}、状態一致 {ss}、節点数一致 {nn}、"
          f"ずらし幾何平均 {sgm([t(x) for p, x, y in pairs]):.1f} / {sgm([t(y) for p, x, y in pairs]):.1f} 秒、最適値の食い違い {bad}")
    for p, x, y in pairs:
        if st(x) != st(y) or abs(math.log((t(x) + 1) / (t(y) + 1))) > math.log(1.5):
            print(f"  {p}: {st(x)[:4]} {t(x):.1f}s n{x.get('nodes')} / {st(y)[:4]} {t(y):.1f}s n{y.get('nodes')}")
