"""M の候補選び: python scan.py K N (Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) のうち K 番目 (N 分割) を、既定と
ENOMOTO_MIP_INTSING=1 で前処理だけかけ (ENOMOTO_MIP_DEBUG_STOP_PRESOLVED、1 スレッド、300 秒まで)、前処理の最後の行の数・列の数を比べて
scan_sK.jsonl に足す。途中から再開できる。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
k, N = int(sys.argv[1]), int(sys.argv[2])
probs = sorted({x for part in json.loads((D.parent / "cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})[k::N]
OUT = D / f"scan_s{k}.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()} if OUT.exists() else set()
def pre(mps, extra):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(extra, ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3="/dev/null", ENOMOTO_MIP_DEBUG_STOP_PRESOLVED="1")
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "300", "--threads", "1"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=600)
        err = r.stderr
    except subprocess.TimeoutExpired:
        return None, round(time.time() - t0, 1)
    m = re.findall(r"presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+)", err)
    return (list(map(int, m[-1])) if m else None), round(time.time() - t0, 1)
for p in probs:
    if p in done: continue
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "error": str(e)[:200]}) + "\n")
        continue
    a, ta = pre(mps, {}); b, tb = pre(mps, {"ENOMOTO_MIP_INTSING": "1"})
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "def": a, "intsing": b, "secs": [ta, tb], "changed": a != b}) + "\n")
    print(p, a, b, a != b, flush=True)
