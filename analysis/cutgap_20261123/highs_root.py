"""HiGHS (highspy 1.15.1、または HSEP_PY の改造版) の根の下界の推移: python highs_root.py 問題 [名札] [オプション=値 ...]
mip_max_nodes = 1・threads 1・mip_rel_gap 0・600 秒。ログ (logs/highs_<問題>_<名札>.log) の表のノード 0 の行の dual bound を拾う。
WRITE_PRESOLVED=1 なら前処理後の問題を presolved/<問題>.mps に書く (当方に解かせて前処理の差を切り分ける)。
表示の dual bound は元の目的 (前処理の定数込み)。結果は highs.jsonl に 1 行。"""
import json, os, re, sys, time
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent; WT = D.parents[1]
p = sys.argv[1]; tag = sys.argv[2] if len(sys.argv) > 2 else "def"; extra = sys.argv[3:]
(D / "logs").mkdir(exist_ok=True); LOG = D / "logs" / f"highs_{p}_{tag}.log"
if LOG.exists(): LOG.unlink()
h = highspy.Highs(); h.setOptionValue("output_flag", False)
for k, v in (("mip_max_nodes", int(os.environ.get("HIGHS_NODES", "1"))), ("time_limit", 600.0), ("threads", 1), ("mip_rel_gap", 0.0), ("log_file", str(LOG)), ("log_to_console", False), ("output_flag", True)):
    h.setOptionValue(k, v)
for e in extra:
    k, v = e.split("=", 1); h.setOptionValue(k, type(h.getOptionValue(k))(v) if not isinstance(h.getOptionValue(k), bool) else v.lower() in ("1", "true", "on"))
h.readModel(str(WT / ".miplib_cache" / f"{p}.mps"))
if os.environ.get("WRITE_PRESOLVED"):
    h.presolve(); (D / "presolved").mkdir(exist_ok=True); h.writePresolvedModel(str(D / "presolved" / f"{p}.mps"))
    h.clearSolver()
t = time.perf_counter(); h.run(); t = time.perf_counter() - t
txt = LOG.read_text(); rows = []
for l in txt.splitlines():
    f = l.split()
    if len(f) >= 11 and re.fullmatch(r"\d+(\.\d+)?s", f[-1]):
        if f[0].isalpha() and len(f[0]) <= 2: f = f[1:]
        try:
            if int(f[0]) == 0: rows.append([float(f[-1][:-1]), f[4], f[5], int(f[-4]), int(f[-3]) if f[-3].isdigit() else f[-3]])
        except ValueError: pass
m = re.search(r"Presolve.*?:\s*(\d+) rows, (\d+) cols", txt)
info = h.getInfo()
row = {"problem": p, "tag": tag, "extra": extra, "status": h.modelStatusToString(h.getModelStatus()), "dual": info.mip_dual_bound,
       "primal": info.objective_function_value, "root": rows, "presolved": re.findall(r"Presolved problem has (\d+) rows; (\d+) cols", txt) or re.findall(r"Presolving model\n(\d+) rows, (\d+) cols", txt), "time": round(t, 2)}
print(json.dumps(row)[:800])
with open(D / "highs.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
