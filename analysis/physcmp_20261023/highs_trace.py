"""HiGHS をログつきで解き、下界・暫定解の推移とカットプールを取る。
python highs_trace.py <mps> <秒> <出力の頭> [キー=値 ...]
<頭>.log: HiGHS のログ (各行に経過秒)、<頭>.trace: 下界・暫定解が変わった時刻、<頭>.cuts: カットプール (コールバック毎の本数・非零・長さの分布)
最後の行に JSON。"""
import json, sys, time
import highspy
from highspy import cb

mps, T, out = sys.argv[1], float(sys.argv[2]), sys.argv[3]
h = highspy.Highs()
h.setOptionValue("time_limit", T)
h.setOptionValue("threads", 4)
h.setOptionValue("mip_rel_gap", 0.0)
h.setOptionValue("output_flag", True)
h.setOptionValue("log_to_console", False)
for kv in sys.argv[4:]:
    k, v = kv.split("=")
    for cast in (int, float, str):
        try:
            if h.setOptionValue(k, cast(v)) == highspy.HighsStatus.kOk:
                break
        except Exception:
            pass
t0 = time.perf_counter()
logf = open(out + ".log", "w")
trf = open(out + ".trace", "w")
cutf = open(out + ".cuts", "w")
last = [None, None]
import os
dump_cuts = os.environ.get("DUMP_CUTS") is not None
pools = []

def on_cb(ctype, msg, dout, din, user):
    el = time.perf_counter() - t0
    if ctype == cb.HighsCallbackType.kCallbackLogging:
        for line in msg.rstrip("\n").split("\n"):
            logf.write(f"[{el:7.2f}] {line}\n")
        logf.flush()
        return
    if ctype == cb.HighsCallbackType.kCallbackMipGetCutPool:
        n = dout.cutpool_num_cut
        st = list(dout.cutpool_start)
        lens = [int(st[i + 1] - st[i]) for i in range(n)] if n else []
        hist = {}
        for l in lens:
            b = 2 if l <= 2 else (3 if l <= 3 else (5 if l <= 5 else (10 if l <= 10 else (50 if l <= 50 else 999))))
            hist[b] = hist.get(b, 0) + 1
        if dump_cuts:
            json.dump({"num_col": int(dout.cutpool_num_col), "start": [int(v) for v in st], "index": [int(v) for v in dout.cutpool_index],
                       "value": [float(v) for v in dout.cutpool_value], "lower": [float(v) for v in dout.cutpool_lower],
                       "upper": [float(v) for v in dout.cutpool_upper]}, open(out + f".pool{len(pools)}.json", "w"))
            pools.append(1)
        cutf.write(json.dumps({"t": round(el, 2), "rt": dout.running_time, "db": dout.mip_dual_bound, "pb": dout.mip_primal_bound,
                               "ncut": int(n), "nnz": sum(lens), "len_hist": hist}) + "\n")
        cutf.flush()
        return
    db, pb = dout.mip_dual_bound, dout.mip_primal_bound
    if (db, pb) != tuple(last):
        last[0], last[1] = db, pb
        trf.write(f"{el:.2f} rt={dout.running_time:.2f} type={int(ctype)} dual={db} primal={pb} nodes={dout.mip_node_count} it={dout.simplex_iteration_count}\n")
        trf.flush()

h.setCallback(on_cb, None)
for c in (cb.HighsCallbackType.kCallbackLogging, cb.HighsCallbackType.kCallbackMipInterrupt, cb.HighsCallbackType.kCallbackMipGetCutPool,
          cb.HighsCallbackType.kCallbackMipImprovingSolution, cb.HighsCallbackType.kCallbackMipLogging):
    h.startCallback(c)
h.readModel(mps)
t0 = time.perf_counter()
h.run()
t = time.perf_counter() - t0
info = h.getInfo()
print(json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": info.objective_function_value,
                  "bound": info.mip_dual_bound, "nodes": info.mip_node_count, "time": t}), flush=True)
