"""fuzz_tiny の問題を ENOMOTO・HiGHS・全列挙 (brute.py) で解いて比べる。
使い方: python fz_eval.py 問題数 [開始seed] > 結果.txt  (ENOMOTO のログは標準エラー)
全列挙: 0-1 列を全部試し、連続列 (幅 <= 2e-9、目的への寄与 6e-7 以下) は 0 に置く。tol = 行の違反 <= 1e-6 の最良、exact = 違反 0 の最良。"""
import sys, os, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
_argv = list(sys.argv)
import brute  # (fuzz_tiny_cls.py の頭が sys.argv を書き換えるので先に写す)
sys.argv = _argv
ns = brute.ns
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
same = lambda a, b: a is not None and b is not None and abs(a - b) <= 1e-6 * (1 + abs(a))
cnt = {"ours!=highs": 0, "ours!=tol": 0, "highs!=tol": 0, "ours worse than tol": 0, "ours better than tol": 0}
t0 = time.time()
for seed in range(start, start + N):
    lp = ns["gen"](seed); ho, hx = ns["hsol"](lp); st, oo, ox = ns["ours"](lp)
    bt, be = brute.best(lp, 1e-6), brute.best(lp, 0.0)
    oo = oo if st == "optimal" else None
    flags = []
    if not same(oo, ho): cnt["ours!=highs"] += 1; flags.append("ours!=highs")
    if not same(oo, bt):
        cnt["ours!=tol"] += 1; flags.append("ours!=tol")
        k = "ours worse than tol" if oo is None or oo > bt else "ours better than tol"; cnt[k] += 1; flags.append(k)
    if not same(ho, bt): cnt["highs!=tol"] += 1; flags.append("highs!=tol")
    if flags:
        ov = ns["viol"](lp, ox) if ox else None
        print(seed, "ours", st, oo, "viol", ov, "highs", ho, "tol", bt, "exact", be, "|", ", ".join(flags), flush=True)
print(cnt, "time %.1f" % (time.time() - t0))
