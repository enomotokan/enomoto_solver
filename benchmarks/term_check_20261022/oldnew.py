"""既定の設定 (環境変数なし) で、修正あり (new) と以前の判定 (old) を交互に解く。使い方: python oldnew.py OUT.jsonl TL REPS PROB..."""
import json, os, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; CACHE = ROOT / ".miplib_cache"
OLD = {"ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT": "1", "ENOMOTO_MIP_NO_LIMIT_GAP_EXIT": "1"}
out = Path(sys.argv[1]); tl = sys.argv[2]; reps = int(sys.argv[3])
logdir = out.with_suffix(""); logdir.mkdir(exist_ok=True)
for name in sys.argv[4:]:
    for rep in range(reps):
        for lab, extra in (("new", {}), ("old", OLD)):
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_") or os.environ.get("KEEP_ENOMOTO")}
            env.update(extra); env["ENOMOTO_MIP_LOG"] = "1"
            p = subprocess.run([sys.executable, str(ROOT / "benchmarks/psched_ioeq_20261022/run.py"), "--worker", str(CACHE / f"{name}.mps"), tl, "4"], capture_output=True, text=True, env=env)
            r = next((json.loads(l) for l in reversed(p.stdout.splitlines()) if l.startswith("{")), {"status": f"crash {p.returncode}"})
            (logdir / f"{name}_{lab}_{rep}.log").write_text(p.stderr)
            r.update(problem=name, label=lab, rep=rep, msgs=[l[:220] for l in p.stderr.splitlines() if "closed the gap" in l or "reporting the search as complete" in l][:3])
            with open(out, "a") as f: f.write(json.dumps(r) + "\n")
            print(name, lab, rep, r["status"], r.get("objective"), r.get("bound"), r.get("root_bound"), round(r.get("time", 0), 1), r["msgs"][:2], flush=True)
