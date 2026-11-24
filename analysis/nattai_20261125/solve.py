"""neos-5188808-nattai を 1 つの設定で解き、最後の暫定解 x を JSON に書き出す (2026-11-25、combo 分担 2 の食い違いの確かめ)。
使い方: python solve.py <cfg> <秒> [追加の環境変数 K=V ...]
cfg は combo_20261124/cfg.py の名前 (def_t1・combo_t1 など)。ログ (log=True) は標準エラーに出る。"""
import json, os, sys, time
from pathlib import Path
R = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(R / "analysis/combo_20261124"))
from cfg import CFGS
cfg, tl = sys.argv[1], float(sys.argv[2])
env = dict(CFGS[cfg]); env.update(kv.split("=", 1) for kv in sys.argv[3:])
os.environ.update(env)
from enomoto_solver import benchmark_highs as bh
m, _, _, names = bh._read_our_model(R / ".miplib_cache/neos-5188808-nattai.mps")
th = 4 if cfg.endswith("_t4") else 1
t = time.perf_counter()
r = m.solve(None, False, tl, None, None, th, True, None)
t = time.perf_counter() - t
tag = sys.argv[3:] and "_" + "_".join(a.replace("ENOMOTO_MIP_", "").replace("=", "") for a in sys.argv[3:]) or ""
out = {"cfg": cfg, "env": env, "status": r["status"], "objective": r["objective"], "bound": r["best_bound"],
       "nodes": r["nodes"], "time": t, "x": dict(zip(names, list(r["x"]))) if r.get("x") is not None else None}
Path(f"x_{cfg}{tag}.json").write_text(json.dumps(out))
print(json.dumps({k: v for k, v in out.items() if k != "x"}))
