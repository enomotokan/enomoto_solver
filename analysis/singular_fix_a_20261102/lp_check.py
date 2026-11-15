"""LP の答えが変わらないことの確かめ (2026-11-02)。既定 (base) と試作 (rep: ENOMOTO_SIMPLEX_WARM_SING_REPAIR=1) で
LP を解き、HiGHS の目的関数値と相対 1e-6 で比べる。Netlib の取得元 (netlib.org) にこの環境から届かないので、
HiGHS のリポジトリの check/instances にある Netlib の問題を使う。

使い方: python lp_check.py <MPS のディレクトリ> <出力の jsonl> [問題名 ...]
"""
import json
import os
import subprocess
import sys
from pathlib import Path

WT = Path(__file__).resolve().parents[2]
NETLIB = ["25fv47", "80bau3b", "adlittle", "afiro", "e226", "etamacro", "greenbea", "israel", "perold", "scrs8", "shell",
          "stair", "standata", "standgub", "standmps", "woodinfe", "forest6", "gas11"]
WORKER = r"""
import sys, json, time
sys.path.insert(0, sys.argv[2])
import highspy
from run_mittelmann_benchmark import _build_our_model
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp()
t = time.perf_counter()
out = _build_our_model(lp).solve(root_solver=None)
t = time.perf_counter() - t
h2 = highspy.Highs(); h2.setOptionValue("output_flag", False); h2.readModel(sys.argv[1]); h2.run()
print("RESULT " + json.dumps({"status": out["status"], "obj": out.get("objective"), "time": t,
      "highs_status": h2.modelStatusToString(h2.getModelStatus()), "highs_obj": h2.getInfo().objective_function_value}))
"""
d, outp = Path(sys.argv[1]), Path(sys.argv[2])
names = sys.argv[3:] or NETLIB
for p in names:
    f = d / f"{p}.mps"
    if not f.exists():
        continue
    for cfg, extra in [("base", {}), ("rep", {"ENOMOTO_SIMPLEX_WARM_SING_REPAIR": "1"})]:
        env = dict(os.environ, **extra)
        r = subprocess.run([sys.executable, "-c", WORKER, str(f), str(WT / "scripts")], env=env, capture_output=True, text=True, timeout=600)
        line = [l for l in r.stdout.splitlines() if l.startswith("RESULT ")]
        res = json.loads(line[-1][7:]) if line else {"status": f"crash rc={r.returncode}", "err": r.stderr[-300:]}
        ok = None
        if isinstance(res.get("obj"), (int, float)) and isinstance(res.get("highs_obj"), (int, float)):
            ok = abs(res["obj"] - res["highs_obj"]) <= 1e-6 * max(1.0, abs(res["highs_obj"]))
        res.update(problem=p, cfg=cfg, match=ok)
        with open(outp, "a") as fo:
            fo.write(json.dumps(res) + "\n")
        print(p, cfg, res.get("status"), res.get("obj"), res.get("highs_obj"), ok, flush=True)
