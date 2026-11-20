"""再スタートの閾値に届かない固定が多い問題で、木に入るときの大域の境界で冗長な行・非零がどれだけ LP に残るかを数える (走査だけ、試作なし)。
P (MIPLIB 58 問、shards_P.json) と M (Mittelmann 186 問、cbcimpl_20261029/mitt_parts.json) を 4 スレッド・種 1・40 秒で回し、
ログの「restart check: newly fixed N … threshold T」(最後の 1 行) と、診断 ENOMOTO_MIP_DIAG_ROWTYPES の「ROWTYPES tree start」の行を拾う。
python redscan.py → REDSCAN_OUT (既定 redscan.jsonl、途中から再開できる)"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = sorted({x[0] for s in json.loads((D / "shards_P.json").read_text())["shards"] for x in s})
M = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P]
OUT = Path(os.environ.get("REDSCAN_OUT", str(D / "redscan.jsonl")))
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
TL = 40
for s, p in probs:
    if p in done: continue
    row = {"set": s, "problem": p}
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        row["error"] = str(e)[:200]
    else:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_DIAG_ROWTYPES": "1",
                    "ENOMOTO_T_MIP_DIAG_ROWTYPES_SECS": "1e9"})
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 300)
            out = pr.stderr
            try: row["status"] = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1]).get("status")
            except Exception: row["status"] = "crash"  # noqa: BLE001
        except subprocess.TimeoutExpired as e:
            out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""); row["status"] = "timeout"
        rc = re.findall(r"restart check: newly fixed (\d+), integer columns (\d+), free at the root (\d+), threshold ([\d.]+)", out)
        row["restart_checks"] = [[int(a), int(b), int(c), float(d)] for a, b, c, d in rc]
        ts = re.findall(r"MIP: ROWTYPES tree start: (.*)", out)
        if ts:
            l = ts[-1]
            m = re.search(r"rows (\d+) nnz (\d+) fixed int (\d+)/(\d+)", l)
            red = re.findall(r"red (\d+) \(nnz (\d+)\)", l)
            row.update({"rows": int(m[1]), "nnz": int(m[2]), "fixed_int": int(m[3]), "nint": int(m[4]),
                        "red_rows": sum(int(a) for a, _ in red), "red_nnz": sum(int(b) for _, b in red), "rowtypes": l[:600]})
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps({k: v for k, v in row.items() if k != "rowtypes"}), flush=True)
