"""Model.solve(seed=...) の確かめ (analysis/seedarg_20261124/README.md): 1 スレッドで、種を環境変数 ENOMOTO_T_MIP_SEED と引数 seed で与えた
ノード数・目的値を並べる。使い方: python check.py <env|argN|none> <秒> <問題 ...>
(env・none は引数なし (env は環境変数を与えて回す前提)、argN は seed=N。none は変更前の版でも回る)。1 行 1 問の JSON を標準出力へ。リポジトリの根で回す"""
import json, sys, time
from enomoto_solver import Model

mode, tl, probs = sys.argv[1], float(sys.argv[2]), sys.argv[3:]
seed = int(mode[3:]) if mode.startswith("arg") else None
for p in probs:
    M = Model()
    M.readModel(f".miplib_cache/{p}.mps")
    t = time.perf_counter()
    kw = {} if seed is None else {"seed": seed}  # 変更前の版 (seed の引数なし) でも回せるように
    s = M.solve(threads=1, time_limit=tl, log=False, **kw)
    print(json.dumps({"problem": p, "mode": mode, "seed": seed, "status": s.status, "objective": s.objective, "nodes": s.nodes, "time": round(time.perf_counter() - t, 2)}), flush=True)
