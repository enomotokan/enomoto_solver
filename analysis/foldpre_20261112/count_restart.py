"""再スタート・サブ MIP の後の問題で畳める量を数える (ENOMOTO_MIP_FOLD_COUNT=1、1 スレッドで解く。analysis/foldpre_20261112.md)。
python count_restart.py <出力.jsonl> <時間> <問題 ...>   (wheel は FOLD_PKG)"""
import json, os, subprocess, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
out, tl, probs = sys.argv[1], sys.argv[2], sys.argv[3:]
done = set()
if os.path.exists(out):
    done = {json.loads(l)["problem"] for l in open(out)}
for p in probs:
    if p in done:
        continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    if os.environ.get("FOLD_PKG"):
        env["PYTHONPATH"] = os.environ["FOLD_PKG"]
    env.update(ENOMOTO_MIP_FOLD_COUNT="1", ENOMOTO_MIP_THREADS="1")
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                        "--time-limit", tl, "--threads", "1"], env=env, capture_output=True, text=True, timeout=int(tl) + 600)
    lines = [l for l in r.stderr.splitlines() if l.startswith("MIP_FOLD_COUNT") and "node" not in l]
    with open(out, "a") as f:
        f.write(json.dumps({"problem": p, "lines": lines[:200], "n_lines": len(lines), "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, len(lines), [l for l in lines if "restart" in l][:3], flush=True)
