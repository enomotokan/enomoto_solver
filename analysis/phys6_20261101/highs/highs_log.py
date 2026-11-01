"""HiGHS (highspy) をログつきで解く (2026-11-01、physcmp_20261023/highs_trace.py からカットプールを除いたもの)。
python highs_log.py <mps> <秒> <出力の頭> [キー=値 ...]
<頭>.log: HiGHS のログ (各行に経過秒)。<頭>.trace: 下界・暫定解が変わった時刻 (コールバックの種類・ノード数・反復数つき)。
最後の行に JSON。4 スレッド・mip_rel_gap=0。"""
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
last = [None, None]


def on_cb(ctype, msg, dout, din, user):
    el = time.perf_counter() - t0
    if ctype == cb.HighsCallbackType.kCallbackLogging:
        for line in msg.rstrip("\n").split("\n"):
            logf.write(f"[{el:7.2f}] {line}\n")
        logf.flush()
        return
    db, pb = dout.mip_dual_bound, dout.mip_primal_bound
    if (db, pb) != tuple(last):
        last[0], last[1] = db, pb
        trf.write(f"{el:.2f} rt={dout.running_time:.2f} type={int(ctype)} dual={db} primal={pb} nodes={dout.mip_node_count} it={dout.simplex_iteration_count}\n")
        trf.flush()


h.setCallback(on_cb, None)
for c in (cb.HighsCallbackType.kCallbackLogging, cb.HighsCallbackType.kCallbackMipInterrupt,
          cb.HighsCallbackType.kCallbackMipImprovingSolution, cb.HighsCallbackType.kCallbackMipLogging):
    h.startCallback(c)
h.readModel(mps)
t0 = time.perf_counter()
h.run()
t = time.perf_counter() - t0
info = h.getInfo()
print(json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": info.objective_function_value,
                  "bound": info.mip_dual_bound, "nodes": info.mip_node_count, "time": t, "opts": sys.argv[4:]}), flush=True)
