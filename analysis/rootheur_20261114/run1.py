"""1 回解いてログを取る: python run1.py 問題 設定 種 スレッド数 秒 ログ [nl=ノード上限] [K=V ...]。
設定は analysis/cand1114_20261113/cfg.py の def か cand1114 (+ cfg.py の試作の組)。ENOMOTO_MIP_DEBUG_RHSTAT=1 を付ける。
ログ (標準エラー) の最後に RESULT 行 (状態・目的値・下界・ノード・根の下界・秒)。ログは gzip で書く。"""
import gzip, os, sys, subprocess
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
p, cfg, seed, th, tl, log = sys.argv[1:7]
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = th; env["ENOMOTO_T_MIP_SEED"] = seed
env["ENOMOTO_MIP_DEBUG_RHSTAT"] = "1"
nl = "None"
for kv in sys.argv[7:]:
    k, v = kv.split("=", 1)
    if k == "nl": nl = v
    else: env[k] = v
code = f"""
import time, sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(WT / '.miplib_cache' / (p + '.mps'))!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, {float(tl)}, None, {nl}, {th}, True); t = time.perf_counter() - t
print('RESULT', r['status'], r['objective'], r['best_bound'], r['nodes'], r.get('root_bound'), round(t, 2), file=sys.stderr)
"""
pr = subprocess.run([sys.executable, "-c", code], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=float(tl) * 1.5 + 300)
with gzip.open(log, "wb") as f:
    f.write(pr.stdout)
