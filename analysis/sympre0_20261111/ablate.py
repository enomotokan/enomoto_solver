"""失った列のある問題で、前処理の段を 1 つずつ止めて失う列が戻るかを見る (2026-11-11)。
使い方: python ablate.py 問題,... → ablate.jsonl に追記"""
import json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
OFFS = {"none": {}, "clqext": {"ENOMOTO_MIP_NO_CLIQUE_EXTEND": "1"}, "probing": {"ENOMOTO_MIP_NO_PROBING": "1"},
        "coeftight": {"ENOMOTO_MIP_NO_COEF_TIGHTEN": "1"}, "lpprobe": {"ENOMOTO_MIP_NO_LP_PROBE": "1"},
        "rowmerge": {"ENOMOTO_MIP_NO_ROW_MERGE": "1"}}
probs = sys.argv[1].split(",")
offs = sys.argv[2].split(",") if len(sys.argv) > 2 else list(OFFS)
def one(p, o):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM="1", ENOMOTO_MIP_SYMPRE0_DIAG="1", **OFFS[o])
    try:
        err = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", "600", "--threads", "1"],
                             cwd=WT, env=env, capture_output=True, text=True, timeout=600).stderr
    except subprocess.TimeoutExpired:
        err = ""
    m = re.search(r"still free (\d+), lost (\d+) \((\d+) integer\), gained (\d+)", err)
    a = re.search(r"SYMPRE0 after: .*?generators (\d+), moved (\d+) .*?orbits (\d+) on (\d+) columns", err)
    return {"problem": p, "off": o, "cmp": list(map(int, m.groups())) if m else None, "after": list(map(int, a.groups())) if a else None}
with ThreadPoolExecutor(3) as ex:
    for r in ex.map(lambda a: one(*a), [(p, o) for p in probs for o in offs]):
        with open(D / "ablate.jsonl", "a") as f: f.write(json.dumps(r) + "\n")
        print(r, flush=True)
