"""bench.jsonl (def・bs) を MIPLIB の solu (=opt=) と比べ、下界が最適値を越えないか・解けた数・下界の変化を表にする (2026-11-04)。
使い方: python analysis/bound_step_20261104/judge.py [bench.jsonl]   → 表を標準出力に、判定を <入力の名前>_judge.jsonl に書く。
目的値の刻みは元の MPS (highspy で読む) から ENOMOTO と同じ規則 ([`MipProblem::objective_step`]: 費用のある列が全て整数なら、
係数を 1,2,...,1000 倍して整数になる最小の倍率での gcd / 倍率) で求める (参考。ソルバーの中の刻みは前処理後の問題で求める)。"""
import json, math, sys
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "mittelmann_ready_20261103"))
sys.path.insert(0, str(WT / "scripts"))
from check_sol import load_solu  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402


def info(p):
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    h.readModel(str(rb.fetch(p, WT / ".miplib_cache")))
    lp = h.getLp()
    mx = lp.sense_ == highspy.ObjSense.kMaximize
    integ = list(lp.integrality_) if len(lp.integrality_) else [highspy.HighsVarType.kContinuous] * lp.num_col_
    cs = [abs(c) for j, c in enumerate(lp.col_cost_) if c != 0]
    if any(c != 0 and integ[j] != highspy.HighsVarType.kInteger for j, c in enumerate(lp.col_cost_)) or not cs:
        return mx, None
    for mult in [1, 2, 3, 4, 5, 6, 8, 10, 12, 100, 1000]:
        sc = [c * mult for c in cs]
        if all(abs(s - round(s)) <= 1e-9 * max(s, 1) for s in sc):
            g = 0
            for s in sc:
                g = math.gcd(g, int(round(s)))
            if g > 0:
                return mx, g / mult
    return mx, None


f = Path(sys.argv[1]) if len(sys.argv) > 1 else D / "bench.jsonl"
recs = [json.loads(l) for l in f.read_text().splitlines()]
solu = load_solu()
by = {}
for r in recs:
    by.setdefault(r["problem"], {})[r["cfg"]] = r["result"]
out = []
CF = list(dict.fromkeys(r["cfg"] for r in recs))
cnt = {c: 0 for c in CF}
bad = []
print(f"{'problem':28} {'step':>6} {'opt':>14} | " + " | ".join(f"{c + ' st':>10} {c + ' bound':>14}" for c in CF) + " | note")
for p, d in by.items():
    mx, step = info(p)
    kind, val = solu.get(p, ("unknown", None))
    row = {"problem": p, "maximize": mx, "step_orig": step, "ref": [kind, val]}
    notes = []
    for c in CF:
        res = d.get(c, {})
        st, b = res.get("status"), res.get("bound")
        if st == "optimal":
            cnt[c] += 1
        row[c] = {"status": st, "objective": res.get("objective"), "bound": b}
        if kind == "opt" and b is not None and math.isfinite(b):
            tol = 1e-6 * max(1.0, abs(val))
            if (b < val - tol) if mx else (b > val + tol):
                notes.append(f"{c}:bound_invalid"); bad.append((p, c))
    bd, bb = row[CF[0]]["bound"], row[CF[-1]]["bound"]
    if bd is not None and bb is not None and bd != bb and row[CF[0]]["status"] != "optimal" and row[CF[-1]]["status"] != "optimal":
        notes.append("bound_changed")
    row["notes"] = notes
    out.append(row)
    fmt = lambda v: "-" if v is None else f"{v:.8g}"
    print(f"{p:28} {fmt(step):>6} {fmt(val):>14} | " + " | ".join(f"{str(row[c]['status']):>10} {fmt(row[c]['bound']):>14}" for c in CF) + f" | {' '.join(notes)}")
print("optimal:", cnt, "bound_invalid:", bad)
(D / (f.stem + "_judge.jsonl")).write_text("".join(json.dumps(r) + "\n" for r in out))
