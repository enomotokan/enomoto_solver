"""打ち切り値 (ENOMOTO_MIP_CUTOFF) を最適値のすぐ下 (below: 答えは実行不能) / すぐ上 (above: 実行不能と言ってはいけない) に置き、
また最適解を開始解に与えて (start)、修正あり (new) と以前の判定 (old: ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT・ENOMOTO_MIP_NO_LIMIT_GAP_EXIT) で解く。
使い方: python cutoff_check.py OUT.jsonl TL PROB:OPT ..."""
import json, os, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; CACHE = ROOT / ".miplib_cache"
sys.path.insert(0, str(Path(__file__).resolve().parent)); import startfile_lib as SL
OLD = {"ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT": "1", "ENOMOTO_MIP_NO_LIMIT_GAP_EXIT": "1"}
out = Path(sys.argv[1]); tl = sys.argv[2]
for arg in sys.argv[3:]:
    name, opt = arg.split(":"); opt = float(opt); d = max(1e-4 * abs(opt), 1e-3)
    sol = SL.opt_solution(name)
    cases = [("below", {"ENOMOTO_MIP_CUTOFF": repr(opt - d)}), ("above", {"ENOMOTO_MIP_CUTOFF": repr(opt + d)})]
    if sol: cases.append(("start", {"ENOMOTO_MIP_START_FILE": str(sol)}))
    for case, ev in cases:
        for lab, extra in (("old", OLD), ("new", {})):
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ev); env.update(extra); env["ENOMOTO_MIP_LOG"] = "1"
            p = subprocess.run([sys.executable, str(ROOT / "benchmarks/psched_ioeq_20261022/run.py"), "--worker", str(CACHE / f"{name}.mps"), tl, "4"], capture_output=True, text=True, env=env)
            r = next((json.loads(l) for l in reversed(p.stdout.splitlines()) if l.startswith("{")), {"status": f"crash {p.returncode}", "err": p.stderr[-300:]})
            r.update(problem=name, case=case, label=lab, opt=opt, msgs=[l[:200] for l in p.stderr.splitlines() if "closed the gap" in l or "reporting the search as complete" in l][:3])
            ok = {"below": r["status"] in ("infeasible", "time_limit"), "above": r["status"] != "infeasible" and (r["status"] != "optimal" or abs(r["objective"] - opt) <= 1e-6 * max(1, abs(opt))),
                  "start": r["status"] != "infeasible" and (r["status"] != "optimal" or abs(r["objective"] - opt) <= 1e-6 * max(1, abs(opt)))}[case]
            r["ok"] = ok
            with open(out, "a") as f: f.write(json.dumps(r) + "\n")
            print(name, case, lab, r["status"], r.get("objective"), r.get("bound"), round(r.get("time", 0), 2), "OK" if ok else "BAD", r["msgs"][:1], flush=True)
