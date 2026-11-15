"""Mittelmann の問題で `ENOMOTO_MIP_TREE2_INTAGG=d` が前処理を変えるかを数える (analysis/presolvegap_20261113.md)。解かない:
cand1114 の上で `ENOMOTO_MIP_DEBUG_GCDCOUNT=stop` (前処理の最後の行の数を出して終える) を 1 スレッドで回し、行の数を比べる。
python count_mitt.py <出力.jsonl> <base|d> <問題の一覧のファイル>   (wheel は PYTHONPATH)"""
import json, os, re, subprocess, sys, time
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "analysis" / "cand1114_20261113"))
from cfg import CAND1114  # noqa: E402
out, mode, lst = sys.argv[1], sys.argv[2], sys.argv[3]
done = set()
if os.path.exists(out):
    done = {json.loads(l)["problem"] for l in open(out)}
for p in open(lst).read().split():
    if p in done:
        continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CAND1114, ENOMOTO_MIP_DEBUG_GCDCOUNT="stop", ENOMOTO_MIP_THREADS="1")
    if mode == "d":
        env["ENOMOTO_MIP_TREE2_INTAGG"] = "d"
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                            "--time-limit", "600", "--threads", "1"], env=env, capture_output=True, text=True, timeout=600)
        m = re.search(r"GCDCOUNT final: rows (\d+) ineq (\d+)", r.stderr)
        rows = (int(m.group(1)), int(m.group(2))) if m else None
    except subprocess.TimeoutExpired:
        rows = "timeout"
    with open(out, "a") as f:
        f.write(json.dumps({"problem": p, "mode": mode, "rows": rows, "wall": round(time.time() - t0, 1)}) + "\n")
