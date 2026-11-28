"""PRESIZE240 8 節: 当方を詳しいログつきで 1 回解く (ログは標準エラーに出る)。
使い方: python vlog.py 問題 スレッド 秒 [環境変数=値 ...] 2> ログ"""
import os, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
p, th, tl = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
for kv in sys.argv[4:]:
    k, v = kv.split("=", 1); os.environ[k] = v
os.environ["ENOMOTO_MIP_THREADS"] = str(th); os.environ.setdefault("ENOMOTO_T_MIP_SEED", "1")
from enomoto_solver import benchmark_highs as bh  # noqa: E402
m, _, _, _ = rb._read_our_model_compat(bh, rb.fetch(p, WT / ".miplib_cache"))
t = time.perf_counter(); r = m.solve(None, True, tl, None, None, th)
print(p, r["status"], r["objective"], r["nodes"], round(time.perf_counter() - t, 2), flush=True)
