"""nw04 の根のログを取る: python diag.py 設定 種 スレッド数 [追加の環境変数 K=V ...] > ログ (標準エラーにソルバーのログ)。
設定は cfg.py の cand1114 か pint。"""
import os, sys, time, json, subprocess
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
cfg, seed, th = sys.argv[1], sys.argv[2], sys.argv[3]
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = th; env["ENOMOTO_T_MIP_SEED"] = seed
for kv in sys.argv[4:]:
    k, v = kv.split("=", 1); env[k] = v
code = f"""
import time, sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(WT / '.miplib_cache/nw04.mps')!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, 60.0, None, None, {th}, True); t = time.perf_counter() - t
print('RESULT', r['status'], r['objective'], r['nodes'], round(t, 2), file=sys.stderr)
"""
subprocess.run([sys.executable, "-c", code], env=env, cwd=WT)
