"""DUALEQ2 (2026-11-27): 58 問 (cbcimpl_20261029/probs.py) に ENOMOTO_MIP_DUALEQ=1 をかけ、前処理の DUALEQ の行 (等式にした行の数・
かけ直しで減った行と固定でない列・使ったか) を集める: python pre.py → pre.jsonl。1 スレッド、時間の上限 5 秒 (前処理の行だけ見る)。ログは logs/ (コミットしない)。"""
import json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
sys.path.insert(0, str(R / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
def one(p):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_DUALEQ": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1"})
    r = subprocess.run([sys.executable, str(R / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(R / ".miplib_cache" / f"{p}.mps"), "--time-limit", "5", "--threads", "1"],
                       cwd=R, env=env, capture_output=True, text=True, timeout=600)
    (D / "logs" / f"pre_{p}.log").write_text(r.stderr)
    row = {"problem": p, "made": None, "two": None, "again": None, "used": False}
    m = re.search(r"DUALEQ made (\d+) rows equations \((\d+) with two columns\)", r.stderr)
    if m: row["made"], row["two"] = int(m.group(1)), int(m.group(2))
    m = re.search(r"DUALEQ: presolve again: rows (\d+) -> (\d+), free columns (\d+) -> (\d+)", r.stderr)
    if m:
        row["again"] = [int(x) for x in m.groups()]
        row["used"] = row["again"][1] < row["again"][0] or row["again"][3] < row["again"][2]
    return row
if __name__ == "__main__":
    (D / "logs").mkdir(exist_ok=True)
    with ThreadPoolExecutor(4) as ex:
        rows = list(ex.map(one, PROBS))
    with open(D / "pre.jsonl", "w") as f:
        for r in rows: f.write(json.dumps(r) + "\n")
    for r in rows:
        if r["made"]: print(r)
    print("等式にした問題", sum(1 for r in rows if r["made"]), "使った (縮んだ) 問題", sum(r["used"] for r in rows), "/", len(rows))
