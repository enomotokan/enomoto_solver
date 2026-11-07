"""HiGHS (highspy 1.15.1) の根の下界 (ノード上限 1) を測る。使い方: python highs_root.py 問題 時間 出力 (modkvb_20261107 から写した)
(計装した HiGHS はこの環境ではビルドできなかったので、mod-k を止めた比べはできない。記録の 1 章)"""
import json, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
import highspy
p, tl, out = sys.argv[1], float(sys.argv[2]), sys.argv[3]
mps = rb.fetch(p, WT / ".miplib_cache")
h = highspy.Highs(); h.setOptionValue("output_flag", False)
h.setOptionValue("threads", 4); h.setOptionValue("time_limit", tl); h.setOptionValue("mip_max_nodes", 1)
h.readModel(str(mps))
t0 = time.time(); h.run(); t = time.time() - t0
info = h.getInfo()
rec = {"problem": p, "cfg": "highs", "tl": tl, "root": True, "result": {"status": h.modelStatusToString(h.getModelStatus()), "time": round(t, 2),
       "objective": info.objective_function_value if info.primal_solution_status else None, "bound": info.mip_dual_bound, "nodes": info.mip_node_count},
       "at": time.strftime("%H:%M:%S")}
with open(D / out, "a") as f:
    f.write(json.dumps(rec) + "\n")
print(p, rec["result"], flush=True)
