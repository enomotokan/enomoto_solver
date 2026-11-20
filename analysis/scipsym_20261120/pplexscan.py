"""P (MIPLIB 58 問、analysis/cbcimpl_20261029/probs.py) と M (Mittelmann 186 問、mitt_parts.json) で、試作 `ENOMOTO_MIP_SYM_PPLEX=1` が
働く (成分を格上げする) 問題を洗い出す。前処理と対称性の検出の後で止め (ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3 + STOP_PRESOLVED)、
symmetry の行 (生成元・PPLEX・ORBPP・ORBPP_REPROBE) を拾う。1 スレッド・nice 19・1 問 120 秒まで・3 問並べて回す。
python pplexscan.py → pplexscan.jsonl (途中から再開できる)"""
import json, os, re, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
mitt = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in PROBS] + [("M", p) for p in mitt if p not in PROBS]
OUT = D / "pplexscan.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()

def one(sp):
    s, p = sp
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        return {"set": s, "problem": p, "error": str(e)[:200]}
    dump = Path(tempfile.gettempdir()) / f"pplexscan_{p}.mps"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_SYM_PPLEX": "1",
                "ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3": str(dump), "ENOMOTO_MIP_DEBUG_STOP_PRESOLVED": "1"})
    try:
        pr = subprocess.run(["nice", "-n", "19", sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=200)
        out = pr.stderr
    except subprocess.TimeoutExpired as e:
        out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
    dump.unlink(missing_ok=True)
    g = lambda pat: (re.findall(pat, out) or [None])[0]  # noqa: E731
    return {"set": s, "problem": p, "gens": g(r"MIP: symmetry: (\d+) generators, "), "pplex": g(r"PPLEX: (.*)"), "orbpp": g(r"(ORBPP \d+ of \d+ orbitopes static.*)"),
            "reprobe": g(r"ORBPP_REPROBE: (.*)"), "sst": g(r"SST cuts: (.*)"), "of": g(r"orbital fixing with (.*)"), "stopped": "stopped after the presolve" in out}

with ThreadPoolExecutor(3) as ex:
    for row in ex.map(one, [sp for sp in probs if sp[1] not in done]):
        with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
        print(json.dumps(row), flush=True)
