"""打ち切り値つきの fuzz: scripts/mip_fuzz_vs_highs.py の小さな MIP を HiGHS で解いて最適値 z* を求め、当方に `ENOMOTO_MIP_CUTOFF` を
z* - d (z* より良い解はないので「実行不能」が正しい) と z* + d (「最適 z*」が正しい) で与えて突き合わせる (d = 1e-3 (1 + |z*|))。
HiGHS が実行不能なら、打ち切り値を 0 にして当方も実行不能か確かめる。打ち切り値は環境変数でプロセスの中でキャッシュされるので、
当方は 1 回ずつ別のプロセスで解く。使い方 (リポジトリの根から): python analysis/ti6_20261124/fuzz_cutoff.py 問題数 [開始seed]"""
import os, subprocess, sys, time
src_path = "scripts/mip_fuzz_vs_highs.py"
src = open(src_path).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, src_path, "exec"), ns)
gen, highs, ours = ns["gen"], ns["highs"], ns["ours"]
if len(sys.argv) > 1 and sys.argv[1] == "--one":
    seed = int(sys.argv[2])
    st, z, _ = ours(gen(seed))
    print("RESULT", st, z)
    sys.exit(0)
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; cnt = {}; t0 = time.time()
def run(seed, cut):
    env = dict(os.environ, ENOMOTO_MIP_CUTOFF=repr(cut))
    out = subprocess.run([sys.executable, __file__, "--one", str(seed)], env=env, capture_output=True, text=True).stdout
    line = [l for l in out.splitlines() if l.startswith("RESULT")][-1].split()
    return line[1], (None if line[2] == "None" else float(line[2]))
for seed in range(start, start + N):
    hs, ho = highs(gen(seed))
    if hs == "Optimal":
        d = 1e-3 * (1 + abs(ho))
        cases = [(ho - d, "infeasible", None), (ho + d, "optimal", ho)]
    elif hs == "Infeasible":
        cases = [(0.0, "infeasible", None)]
    else:
        continue
    for cut, want, wz in cases:
        st, z = run(seed, cut)
        ok = st == want and (wz is None or (z is not None and abs(z - wz) <= 1e-6 * (1 + abs(wz))))
        cnt[want] = cnt.get(want, 0) + 1
        if not ok:
            bad += 1
            if bad <= 10:
                print("seed", seed, "cutoff", cut, "want", want, wz, "got", st, z, flush=True)
print("bad", bad, "cases", cnt, "time %.1f" % (time.time() - t0))
