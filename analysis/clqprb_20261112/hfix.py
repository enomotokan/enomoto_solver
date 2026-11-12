"""CLQPRB: HiGHS の前処理で消えた列を「固定」と「代入」に分ける。前処理後の問題の LP 緩和を目的を変えて 4 回解き (1 回ずつ別の
プロセス。同じプロセスで 2 回目の後処理をすると highspy 1.15.1 が落ちる)、後処理で戻した値を比べる。
使い方: python hfix.py 問題 → h/<問題>_hfix.tsv (列番号 種類 値)"""
import sys, subprocess
import numpy as np
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
p = sys.argv[1]
if len(sys.argv) > 2:
    import highspy
    t = int(sys.argv[2])
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1)
    h.readModel(str(WT / ".miplib_cache" / f"{p}.mps"))
    names = list(h.getLp().col_names_)
    h.presolve(); keep = set(h.getPresolvedLp().col_names_)
    lp = h.getPresolvedLp(); lp.integrality_ = []
    if t: lp.col_cost_ = np.random.default_rng(t).normal(size=lp.num_col_)
    q = highspy.Highs(); q.setOptionValue("output_flag", False); q.passModel(lp); q.run()
    s = highspy.HighsSolution(); s.col_value = list(q.getSolution().col_value); s.value_valid = True
    h.postsolve(s)
    np.save(f"/tmp/claude-0/hfix_{p}_{t}.npy", np.array(h.getSolution().col_value))
    np.save(f"/tmp/claude-0/hfix_{p}_keep.npy", np.array([n in keep for n in names]))
    sys.exit(0)
for t in range(4):
    for _ in range(3):
        if subprocess.run([sys.executable, __file__, p, str(t)]).returncode == 0 or Path(f"/tmp/claude-0/hfix_{p}_{t}.npy").exists(): break  # 終わりの後始末で落ちることがある (保存の後なので無視)
X = [np.load(f"/tmp/claude-0/hfix_{p}_{t}.npy") for t in range(4)]
keep = np.load(f"/tmp/claude-0/hfix_{p}_keep.npy")
with open(D / "h" / f"{p}_hfix.tsv", "w") as f:
    for j in range(len(keep)):
        if keep[j]: continue
        k = "fix" if max(abs(X[t][j] - X[0][j]) for t in range(4)) < 1e-9 else "sub"
        f.write(f"{j}\t{k}\t{X[0][j]:g}\n")
