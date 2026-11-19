"""プロービングの含意の種類 (診断 ENOMOTO_MIP_DEBUG_IMPLCLASS) と、衝突グラフの行の上限に当たる行 (診断 ENOMOTO_MIP_DEBUG_CLQTAB の
row_cap_stats、プロービングの前) を、P 58 問 + M (P と重ならない問題) で数える。前処理の後で止める (DUMP_PRESOLVED3 + STOP_PRESOLVED)。
1 スレッド・nice 19・1 問 120 秒まで。python implscan.py K NSHARD → IMPLSCAN_OUT_K (途中から再開できる)"""
import json, os, re, subprocess, sys, tempfile
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = sorted({x[0] for s in json.loads((D / "shards_P.json").read_text())["shards"] for x in s})
M = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P]
k, ns = int(sys.argv[1]), int(sys.argv[2])
probs = probs[k::ns]
OUT = Path(os.environ.get("IMPLSCAN_OUT", str(D / "implscan.jsonl")) + f"_{k}")
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
dump = Path(tempfile.gettempdir()) / f"implscan_dump_{k}.mps"
for s, p in probs:
    if p in done: continue
    row = {"set": s, "problem": p}
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        row["error"] = str(e)[:200]
    else:
        env = {kk: v for kk, v in os.environ.items() if not kk.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_DEBUG_IMPLCLASS": "1", "ENOMOTO_MIP_DEBUG_CLQTAB": "1",
                    "ENOMOTO_T_MIP_CLQTAB_TIME": "1", "ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3": str(dump), "ENOMOTO_MIP_DEBUG_STOP_PRESOLVED": "1"})
        try:
            pr = subprocess.run(["nice", "-n", "19", sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "1"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            out = pr.stderr
        except subprocess.TimeoutExpired as e:
            out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""); row["timeout"] = True
        row["implclass"] = [json.loads(m) for m in re.findall(r"^IMPLCLASS (\{.*\})$", out, re.M)]
        row["clqtab"] = [json.loads(m) for m in re.findall(r"^CLQTAB (\{.*\})$", out, re.M)]
        row["probing"] = re.findall(r"MIP: probing: \d+ of \d+ binaries probed.*", out)[:2]
        row["stopped"] = "stopped after the presolve" in out
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps({kk: v for kk, v in row.items() if kk in ("set", "problem", "stopped", "error", "timeout")}), flush=True)
