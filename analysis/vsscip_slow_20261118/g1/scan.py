"""群 1: P・T・M の各問題で VUBAGG・ORBPP が働くか (オービトープの大きさ) をログから拾う。python scan.py > scan.txt
1 スレッド・20 秒・g1 の設定で 1 回ずつ解き、「VUBAGG n columns」「full orbitopes (...)」「ORBPP k of n」の行を集める。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
M = sorted({x[0] for s in json.loads((D / "shards_M.json").read_text())["shards"] for x in s})
T = sorted({x[0] for s in json.loads((D / "shards_T.json").read_text())["shards"] for x in s})
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_VUBAGG": "1", "ENOMOTO_MIP_ORBPP": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1"})
for set_, ps in (("P", PROBS), ("T", T), ("M", M)):
    for p in ps:
        mps = rb.fetch(p, WT / ".miplib_cache")
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "20", "--threads", "1"],
                               env=env, capture_output=True, text=True, timeout=180)
            err = r.stderr
        except subprocess.TimeoutExpired as e:
            err = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
        vub = re.findall(r"MIP: VUBAGG (\d+) columns", err)
        orbs = re.findall(r"MIP: symmetry: \d+ full orbitopes \(([^)]*)\)", err)
        opp = re.findall(r"ORBPP (\d+) of (\d+) orbitopes static.*fixed (\d+) columns", err)
        if (vub and vub[0] != "0") or orbs or opp:
            print(set_, p, "VUBAGG", vub[:1], "orbitopes", orbs[:2], "ORBPP", opp[:2], flush=True)
