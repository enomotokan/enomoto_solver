"""幅 1e-9 の連続列とクリーク型の行 (`Σx + Σy <= 1 - k 1e-9`) を持つ小さな MIP を作り、HiGHS と最適値を突き合わせる。
使い方: python fuzz_tiny.py 問題数 [開始seed]  (ENOMOTO_MIP_COEF_TIGHTEN_TINY=1 で以前の係数の強化)

判定 (2026-11-09 に直した、analysis/fuzz_tiny_when_20261103.md・analysis/fuzz_tiny_judge_20261109.md): 状態が同じで最適値が
相対 1e-6 で一致すれば正しい。一致しないときは、許容誤差の内の扱いの差 (幅 1e-9 の列・右辺 1 - 2e-9 の行を 1e-6 の内で
どう扱うか) かを次の 3 つで確かめ、全部満たせば「許容誤差の内の差」として正しいとみなす (印字はする):
(1) ENOMOTO の返した点が許容誤差 1e-6 で実行可能 (行・境界・整数性)、(2) 点の目的値が返した最適値と一致、
(3) 返した最適値が、HiGHS を許容誤差 1e-10・前処理なしで解いた値 (厳密に実行可能な最良) 以下 (最小化、相対 1e-6 の余裕)。
(1)〜(3) のどれかが破れたら誤り (bad) と数える。以前の判定 (値の一致だけ) の数も `strict` として出す。"""
import random, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import importlib.util
spec = importlib.util.spec_from_file_location("fz", str(Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py"))
src = open(spec.origin).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, spec.origin, "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); nb = g.randint(3, 10); ny = g.randint(2, 8)
    lo = [0.0] * (nb + ny); up = [1.0] * nb + [1e-9 * g.choice([1, 1, 2]) for _ in range(ny)]
    c = [-round(g.random() * 10 + 1, 2) for _ in range(nb)] + [-round(g.random() * 300, 2) for _ in range(ny)]
    it = [True] * nb + [False] * ny
    rows = []; rlo = []; rup = []
    for _ in range(g.randint(2, 8)):
        xs = g.sample(range(nb), g.randint(1, min(3, nb))); ys = g.sample(range(nb, nb + ny), g.randint(1, min(4, ny)))
        rows.append([(j, 1.0) for j in xs + ys]); rlo.append(-inf); rup.append(1.0 - 1e-9 * g.randint(0, 2))
    for _ in range(g.randint(0, 3)):  # x_a - x_b + y = 0 の形
        a, b = g.sample(range(nb), 2); y = g.randrange(nb, nb + ny)
        rows.append([(a, -1.0), (b, 1.0), (y, 1.0)]); rlo.append(0.0); rup.append(0.0)
    for _ in range(g.randint(1, 3)):  # 普通のナップサック
        xs = g.sample(range(nb), g.randint(2, nb)); rows.append([(j, float(g.randint(1, 9))) for j in xs]); rlo.append(-inf); rup.append(float(g.randint(5, 20)))
    return lo, up, c, it, rows, rlo, rup
def highs_tight(lp):
    """許容誤差 1e-10・前処理なしの HiGHS (厳密に実行可能な最良の値の目安)"""
    import highspy
    lo, up, c, it, rows, rlo, rup = lp; K = highspy.kHighsInf
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    for o in ('primal_feasibility_tolerance', 'mip_feasibility_tolerance'): h.setOptionValue(o, 1e-10)
    h.setOptionValue('presolve', 'off')
    for j in range(len(lo)):
        h.addVar(lo[j], up[j]); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup): h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run(); return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value
def tol_explained(lp, oo, x, tol=1e-6):
    """一致しない最適値が許容誤差の内の扱いの差で説明できるか ((1)〜(3))。理由の文字列も返す"""
    lo, up, c, it, rows, rlo, rup = lp
    if x is None: return False, "点なし"
    for j, v in enumerate(x):
        if v < lo[j] - tol or v > up[j] + tol: return False, f"境界を破る x{j}={v}"
        if it[j] and abs(v - round(v)) > tol: return False, f"整数でない x{j}={v}"
    for r, l, u in zip(rows, rlo, rup):
        a = sum(v * x[j] for j, v in r)
        if a < l - tol or a > u + tol: return False, f"行を破る {a} not in [{l}, {u}]"
    ox = sum(cc * v for cc, v in zip(c, x))
    if abs(ox - oo) > 1e-6 * (1 + abs(oo)): return False, f"点の目的値 {ox} と返した値 {oo} が違う"
    ts, to = highs_tight(lp)
    if ts != "Optimal": return False, f"1e-10 の HiGHS が {ts}"
    if oo > to + 1e-6 * (1 + abs(to)): return False, f"厳密に実行可能な値 {to} より悪い"
    return True, f"許容誤差の内の差 (1e-10 の HiGHS {to})"
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; strict = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible"}.get(hs, hs)
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        strict += 1
        why = ""
        if hmap == os_ == "optimal":
            ok, why = tol_explained(lp, oo, x)
        if not ok:
            bad += 1
            if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, why, flush=True)
        else:
            print("tol", seed, "highs", hs, ho, "ours", os_, oo, why, flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0), f"(strict {strict}: 値の一致だけの以前の判定)")
