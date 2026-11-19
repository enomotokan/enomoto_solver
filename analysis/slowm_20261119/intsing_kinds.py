"""scan_d.jsonl で INTSING (1 形) が前処理を変えた問題について、前処理だけかけ (引数で INTSING の形、既定 1) (1 スレッド、ENOMOTO_MIP_INTSING_LOG)、消した整数列の形
(係数の符号の向き opp/same/mixed・列の境界・費用の有無・右辺・境界から足した行の数) を集めて intsing_kinds.jsonl に書く。"""
import json, os, re, subprocess, sys
MODE = sys.argv[1] if len(sys.argv) > 1 else "1"
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
sd = [json.loads(l) for l in open(D / "scan_d.jsonl") if l.strip()]
out = open(D / ("intsing_kinds.jsonl" if MODE == "1" else f"intsing_kinds_{MODE}.jsonl"), "w")
for r in sd:
    if r["one"] == r["none"]:
        continue
    mps = rb.fetch(r["problem"], WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_INTSING=MODE, ENOMOTO_MIP_INTSING_LOG="1", ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3="/dev/null", ENOMOTO_MIP_DEBUG_STOP_PRESOLVED="1")
    p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "300", "--threads", "1"], cwd=WT, env=env, capture_output=True, text=True, timeout=900)
    logs = [l for l in p.stderr.splitlines() if l.startswith("INTSING_LOG")]
    rec = {"problem": r["problem"], "set": r["set"], "none": r["none"], "one": r["one"], "d": r["d"], "logs": logs}
    out.write(json.dumps(rec, ensure_ascii=False) + "\n"); out.flush()
    print(r["problem"], logs[:1], flush=True)
