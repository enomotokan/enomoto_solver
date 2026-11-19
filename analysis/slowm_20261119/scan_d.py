"""INTSING の 3 つの形 (なし / 1 / d) で前処理だけかけ (1 スレッド)、最後の presolve 行 (行・列) を比べる。P の 58 問と M の分担表の問題。
使い方: python scan_d.py → scan_d.jsonl"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
M = sorted({t[0] for s in json.loads((D / "shards_M.json").read_text())["shards"] for t in s})
OUT = D / "scan_d.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()} if OUT.exists() else set()
def pre(mps, v):
    env = {k: x for k, x in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_INTSING=v, ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3="/dev/null", ENOMOTO_MIP_DEBUG_STOP_PRESOLVED="1")
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "300", "--threads", "1"], cwd=WT, env=env, capture_output=True, text=True, timeout=600)
    except subprocess.TimeoutExpired:
        return None
    m = re.findall(r"presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+)", r.stderr)
    return list(map(int, m[-1])) if m else None
for p in list(PROBS) + [x for x in M if x not in PROBS]:
    if p in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    a, b, c = pre(mps, "0"), pre(mps, "1"), pre(mps, "d")
    with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "set": "P" if p in PROBS else "M", "none": a, "one": b, "d": c}) + "\n")
    print(p, a != b, a != c, flush=True)
