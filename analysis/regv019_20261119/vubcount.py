"""VUBAGG が働く問題を数える (2026-11-19): 既定で 1 スレッド・短い時間 (既定 3 秒) で解き、ログの `MIP: VUBAGG k columns` を拾う。
使い方: python vubcount.py [秒] → vubcount.jsonl (P の 58 問 + Mittelmann 186 問のうち .miplib_cache にあるもの)。"""
import json, os, re, subprocess, sys, concurrent.futures as cf
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
M = [p for part in json.load(open(WT / "analysis/cbcimpl_20261029/mitt_parts.json")) for p in part]
tl = sys.argv[1] if len(sys.argv) > 1 else "3"
probs = [p for p in dict.fromkeys(PROBS + M) if (WT / ".miplib_cache" / f"{p}.mps").exists()]
OUT = D / "vubcount.jsonl"
done = {json.loads(l)["problem"] for l in open(OUT)} if OUT.exists() else set()
def one(p):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")} | {"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_MIP_LOG": "1"}
    try:
        r = subprocess.run([str(WT / ".venv/bin/python"), str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                            "--time-limit", tl, "--threads", "1"], cwd=WT, env=env, capture_output=True, text=True, timeout=600)
        err = r.stderr
    except subprocess.TimeoutExpired as e:
        err = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
    m = re.search(r"MIP: VUBAGG (\d+) columns", err)
    n = re.search(r"MIP_FOLD: n (\d+) ->", err) or re.search(r"rows \d+ -> \d+, free columns (\d+) ->", err)
    rec = {"problem": p, "vub": int(m.group(1)) if m else 0, "n": int(n.group(1)) if n else None}
    with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
    return rec
with cf.ThreadPoolExecutor(4) as ex:
    for rec in ex.map(one, [p for p in probs if p not in done]):
        if rec["vub"]: print(rec, flush=True)
