"""HiGHS (highspy 1.15.1) を 1 回解き、結果を jsonl に足す。部品を止める比べ (アブレーション) 用。
使い方: python hrun.py <問題> <秒> <設定名> [opt=val ...]   (4 スレッド・mip_rel_gap 0、ログは logs/H_<問題>_<設定名>.log)"""
import json, sys, time, os
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent
REPO = D.parents[1]
name, tl, tag = sys.argv[1], float(sys.argv[2]), sys.argv[3]
opts = {}
for kv in sys.argv[4:]:
    k, v = kv.split("=", 1)
    if v in ("true", "false"):
        v = v == "true"
    else:
        try:
            v = int(v)
        except ValueError:
            try:
                v = float(v)
            except ValueError:
                pass
    opts[k] = v
h = highspy.Highs()
# 書いている途中のログを git が書き換えないよう、追跡しない logs/tmp/ に書いてから移す
(D / "logs" / "tmp").mkdir(parents=True, exist_ok=True)
log = D / "logs" / "tmp" / f"H_{name}_{tag}.log"
h.setOptionValue("output_flag", True)
h.setOptionValue("log_to_console", False)
h.setOptionValue("log_file", str(log))
h.setOptionValue("time_limit", tl)
h.setOptionValue("threads", 4)
h.setOptionValue("mip_rel_gap", 0.0)
for k, v in opts.items():
    st = h.setOptionValue(k, v)
    assert st == highspy.HighsStatus.kOk, (k, v)
h.readModel(str(REPO / ".miplib_cache" / f"{name}.mps"))
t0 = time.time()
h.run()
dt = time.time() - t0
info = h.getInfo()
r = dict(prob=name, tag=tag, opts=opts, tl=tl, secs=round(dt, 2), status=h.modelStatusToString(h.getModelStatus()),
         nodes=int(info.mip_node_count), obj=info.objective_function_value, bound=info.mip_dual_bound, gap=info.mip_gap,
         iters=int(info.simplex_iteration_count))
del h
log.replace(D / "logs" / f"H_{name}_{tag}.log")
print(json.dumps(r))
with open(D / "hres.jsonl", "a") as f:
    f.write(json.dumps(r) + "\n")
