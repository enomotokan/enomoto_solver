"""pe (CONFLICT_PROOF_EXPLAIN) が効く問題と悪くなる問題の衝突の型を集める (README §17.5): 問題 × {def, pe} を 4 スレッド・種 1・秒の上限で解き、
衝突解析の統計 (解析の回数・衝突の数・平均長さ・証明から作った衝突・双対証明を遡れずに止まった変更) をログから抜いて conf_type.jsonl に足す。
使い方: [CT_CFG=def,pe,pes] python conf_type.py 秒 問題..."""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CF = {"def": {}, "pe": {"ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN": "1"}, "pes": {"ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN": "1", "ENOMOTO_MIP_PE_SIG": "1"}}
# 環境変数 CT_CFG (カンマ区切り) で回す形を選ぶ (既定は def,pe)
CF = {k: CF[k] for k in os.environ.get("CT_CFG", "def,pe").split(",")}
tl = sys.argv[1]
PAT = {"calls": r"conflict analysis \(HiGHS style\): calls (\d+), conflicts (\d+), added (\d+), mean length ([\d.]+)",
       "proofs": r"dual proofs and conflicts (\d+) \(conflicts added \d+ \(from proofs (\d+)\), Farkas (\d+)",
       "lit": r"literal conflicts: added (\d+), in pool (\d+)",
       "lev": r"conflict levels \(all threads\): examined (\d+), 1-UIP (\d+), stopped with >= 2 left (\d+).*?dual proof (\d+), literal conflict (\d+)",
       "ana": r"conflict analyses \(all threads\): (\d+), stopped: first level not resolved (\d+)"}
for p in sys.argv[2:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c, e in CF.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(e, ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_CONFLICT_LEV_DIAG="1")
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) * 3 + 300)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        rec = {"problem": p, "cfg": c, "status": res.get("status"), "time": res.get("time"), "nodes": res.get("nodes")}
        for k, pat in PAT.items():
            m = re.findall(pat, r.stderr)
            rec[k] = [float(x) for x in m[-1]] if m else None
        with open(D / "conf_type.jsonl", "a") as f:
            f.write(json.dumps(rec) + "\n")
        print(p, c, rec, flush=True)
