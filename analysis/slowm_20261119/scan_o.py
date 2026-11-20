"""INTSING の `o` (支配の固定を元の問題に先にかける) が働く問題を数える: scan_d.jsonl の全問題 (P 58・M) で ENOMOTO_MIP_INTSING=1go の前処理だけかけ、
「dominated binaries before the presolve, fixed」が出るかを scan_o.jsonl に書く。出ない問題は 1g とまったく同じ前処理。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT = open(D / "scan_o.jsonl", "w")
for l in open(D / "scan_d.jsonl"):
    r = json.loads(l)
    env = {k: x for k, x in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_INTSING="1go", ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3="/dev/null", ENOMOTO_MIP_DEBUG_STOP_PRESOLVED="1")
    try:
        p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(rb.fetch(r["problem"], WT / ".miplib_cache")), "--time-limit", "300", "--threads", "1"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=600)
        lines = [x for x in p.stderr.splitlines() if "before the presolve" in x]
    except subprocess.TimeoutExpired:
        lines = ["timeout"]
    rec = {"problem": r["problem"], "set": r["set"], "lines": lines}
    OUT.write(json.dumps(rec) + "\n"); OUT.flush()
    print(r["problem"], lines, flush=True)
