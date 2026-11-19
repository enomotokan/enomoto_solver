"""診断 ENOMOTO_MIP_DIAG_COSTSHIFT (README §14) を scan_d.jsonl の全問題 (P 58・M 32) に、前処理だけ (1 スレッド) かけて scan_cs.jsonl に書く。
引数で INTSING の形を足せる (例 python scan_cs.py 1g → scan_cs_1g.jsonl、INTSING が前処理を変える問題だけ)。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
MODE = sys.argv[1] if len(sys.argv) > 1 else ""
probs = [json.loads(l) for l in open(D / "scan_d.jsonl")]
if MODE:
    ch = {json.loads(l)["problem"] for l in open(D / "intsing_kinds.jsonl")}
    probs = [r for r in probs if r["problem"] in ch]
OUT = open(D / (f"scan_cs_{MODE}.jsonl" if MODE else "scan_cs.jsonl"), "w")
for r in probs:
    env = {k: x for k, x in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_DIAG_COSTSHIFT="1", ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3="/dev/null", ENOMOTO_MIP_DEBUG_STOP_PRESOLVED="1")
    if MODE:
        env["ENOMOTO_MIP_INTSING"] = MODE
    try:
        p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(rb.fetch(r["problem"], WT / ".miplib_cache")), "--time-limit", "300", "--threads", "1"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=900)
        cs = [json.loads(x.split(" ", 1)[1]) for x in p.stderr.splitlines() if x.startswith("COSTSHIFT ")]
        dropped = any("dominated binary fixings dropped" in x for x in p.stderr.splitlines())
    except subprocess.TimeoutExpired:
        cs, dropped = ["timeout"], None
    rec = {"problem": r["problem"], "set": r["set"], "cs": cs, "dropped": dropped}
    OUT.write(json.dumps(rec) + "\n"); OUT.flush()
    print(r["problem"], cs[:1], flush=True)
