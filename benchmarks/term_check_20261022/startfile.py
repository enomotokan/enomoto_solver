"""最適解を ENOMOTO_MIP_START_FILE で与えて解き、根で止まるか・返す値が正しいかを見る (根で暫定解 = 下界の状況を作る)。
使い方: python startfile.py OUT.jsonl TL PROB... (環境変数はそのまま子に渡す。LABEL=... で記録の名前)"""
import json, os, re, subprocess, sys, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; CACHE = ROOT / ".miplib_cache"; SOLS = Path(__file__).resolve().parent / "sols"

def opt_solution(name):
    f = SOLS / f"{name}.sol"
    if f.exists():
        return f
    import highspy
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("time_limit", 300.0)
    h.readModel(str(CACHE / f"{name}.mps")); h.run()
    if h.getModelStatus() != highspy.HighsModelStatus.kOptimal:
        return None
    SOLS.mkdir(exist_ok=True)
    f.write_text("\n".join(repr(v) for v in h.getSolution().col_value) + "\n")
    (SOLS / f"{name}.obj").write_text(repr(h.getInfo().objective_function_value))
    return f

out = Path(sys.argv[1]); tl = sys.argv[2]
for name in sys.argv[3:]:
    sol = opt_solution(name)
    if sol is None:
        print(name, "no optimal solution from HiGHS", flush=True); continue
    env = dict(os.environ, ENOMOTO_MIP_START_FILE=str(sol), ENOMOTO_MIP_LOG="1")
    wk = ROOT / "benchmarks/psched_ioeq_20261022/run.py"
    p = subprocess.run([sys.executable, str(wk), "--worker", str(CACHE / f"{name}.mps"), tl, "4"], capture_output=True, text=True, env=env)
    r = next((json.loads(l) for l in reversed(p.stdout.splitlines()) if l.startswith("{")), {"status": f"crash {p.returncode}", "err": p.stderr[-500:]})
    r["problem"] = name; r["label"] = os.environ.get("LABEL", ""); r["opt"] = float((SOLS / f"{name}.obj").read_text())
    r["closed_msg"] = [l for l in p.stderr.splitlines() if "closed the gap" in l][:2]
    r["start_ok"] = [l for l in p.stderr.splitlines() if "MIP_START_FILE" in l][:1]
    with open(out, "a") as f:
        f.write(json.dumps(r) + "\n")
    print(json.dumps({k: r.get(k) for k in ("problem", "label", "status", "objective", "bound", "opt", "time", "root_bound", "closed_msg")}), flush=True)
