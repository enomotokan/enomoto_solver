"""衝突解析の部品の比べ (README §17): 問題 × {def, pe (CONFLICT_PROOF_EXPLAIN), lg (LIT_EXPLAIN=g), lgpe (両方)} を 1 スレッド・種 1・秒の上限で解き、
時間・ノード・衝突の平均長さを conf_cmp.jsonl に足す。使い方: python conf_cmp.py 秒 問題..."""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CF = {"def": {}, "pe": {"ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN": "1"}, "lg": {"ENOMOTO_MIP_LIT_EXPLAIN": "g"},
      "lgpe": {"ENOMOTO_MIP_LIT_EXPLAIN": "g", "ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN": "1"}}
tl = sys.argv[1]
for p in sys.argv[2:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c, e in CF.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(e, ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1")
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", "1"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) * 3 + 120)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        m = re.search(r"conflict analysis \(HiGHS style\): calls (\d+), conflicts (\d+), added (\d+), mean length ([\d.]+)", r.stderr)
        lv = re.search(r"stopped with >= 2 left (\d+)", r.stderr)
        rec = {"problem": p, "cfg": c, "status": res.get("status"), "time": res.get("time"), "nodes": res.get("nodes"),
               "conf_calls": m and int(m.group(1)), "conf_added": m and int(m.group(3)), "conf_len": m and float(m.group(4))}
        with open(D / "conf_cmp.jsonl", "a") as f:
            f.write(json.dumps(rec) + "\n")
        print(p, c, rec["status"], round(rec["time"] or 0, 1), rec["nodes"], rec["conf_len"], flush=True)
