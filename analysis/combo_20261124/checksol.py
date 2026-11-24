"""最後の暫定解を MPS に代入して、違反を測る (食い違いの確かめ、README 10 節)。

    python analysis/combo_20261124/checksol.py 問題 設定 [秒]

- 設定は cfg.py の名前 (def_t1・combo_t4 など)。run.py と同じ環境変数・スレッド数・種 1 で、ワーカーと同じ呼び方で解く。
- 返ってきた x を、highspy で読んだ元の MPS に代入して次を出す。
  - 目的値 (再計算)
  - 行の違反の最大 (絶対・相対)
  - 列の境界の違反の最大
  - 整数列の非整数性の最大
- ソルバーの中の許容 (1e-6 前後) を超える違反があれば、受け入れた解が実行不能だったことになる。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
if os.environ.get("_CHECKSOL_CHILD") != "1":
    sys.path.insert(0, str(D)); from cfg import CFGS  # noqa: E402
    p, cfg = sys.argv[1], sys.argv[2]; tl = sys.argv[3] if len(sys.argv) > 3 else "600"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "1" if cfg.endswith("_t1") else "4"; env["ENOMOTO_T_MIP_SEED"] = "1"; env["_CHECKSOL_CHILD"] = "1"
    sys.exit(subprocess.run([sys.executable, __file__, p, cfg, tl], env=env, cwd=WT).returncode)

import numpy as np, highspy  # noqa: E402
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
from enomoto_solver import benchmark_highs as bh  # noqa: E402
p, cfg, tl = sys.argv[1], sys.argv[2], float(sys.argv[3])
mps = rb.fetch(p, WT / ".miplib_cache")
m, _, _, names = rb._read_our_model_compat(bh, Path(mps))
th = int(os.environ["ENOMOTO_MIP_THREADS"])
r = m.solve(None, False, tl, None, None, th)
x = np.asarray(r["x"], dtype=float)
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps)); lp = h.getLp()
assert len(x) == lp.num_col_, (len(x), lp.num_col_)
A = lp.a_matrix_; start, idx, val = np.asarray(A.start_), np.asarray(A.index_), np.asarray(A.value_)
act = np.zeros(lp.num_row_)
for j in range(lp.num_col_):
    s, e = start[j], start[j + 1]
    act[idx[s:e]] += val[s:e] * x[j]
lo, up = np.asarray(lp.row_lower_), np.asarray(lp.row_upper_)
rv = np.maximum(np.maximum(lo - act, act - up), 0.0)
rrel = rv / np.maximum(1.0, np.abs(np.where(act < lo, lo, up)))
cl, cu = np.asarray(lp.col_lower_), np.asarray(lp.col_upper_)
cv = np.maximum(np.maximum(cl - x, x - cu), 0.0)
integ = np.array([str(t).endswith("kInteger") for t in lp.integrality_]) if len(lp.integrality_) else np.zeros(lp.num_col_, bool)
iv = np.abs(x - np.round(x)) * integ
obj = float(np.dot(np.asarray(lp.col_cost_), x) + lp.offset_)
i, j, k = int(np.argmax(rv)), int(np.argmax(cv)), int(np.argmax(iv))
print(json.dumps({"problem": p, "cfg": cfg, "status": r["status"], "objective": r["objective"], "objective_recomputed": obj,
                  "row_viol_max": float(rv[i]), "row_viol_rel_max": float(rrel.max()), "row_worst": i, "row_worst_act": float(act[i]), "row_worst_lo": float(lo[i]), "row_worst_up": float(up[i]),
                  "rows_viol_gt_1e-6": int((rrel > 1e-6).sum()), "col_bound_viol_max": float(cv[j]), "int_viol_max": float(iv[k]), "int_worst": names[k] if names else k,
                  "nodes": r["nodes"]}))
