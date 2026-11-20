"""試作 `ENOMOTO_MIP_LPPROBE_TICKS=1` の走査 (pplexscan.py の写し)。P (MIPLIB 58 問) と M (Mittelmann の残り 145 問) で、前処理の後で止め
(ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3 + STOP_PRESOLVED)、LP による固定の行 (「LPPROBE_TICKS gave up」= 試作で早くあきらめた、「root LP … tried N, fixed K」) を拾う。
1 スレッド・nice 19・1 問 120 秒まで・3 問並べて回す。python tkscan.py → tkscan.jsonl (途中から再開できる)"""
import json, os, re, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
mitt = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in PROBS] + [("M", p) for p in mitt if p not in PROBS]
OUT = D / "tkscan.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()

def one(sp):
    s, p = sp
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        return {"set": s, "problem": p, "error": str(e)[:200]}
    dump = Path(tempfile.gettempdir()) / f"tkscan_{p}.mps"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_LPPROBE_TICKS": "1",
                "ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3": str(dump), "ENOMOTO_MIP_DEBUG_STOP_PRESOLVED": "1"})
    try:
        pr = subprocess.run(["nice", "-n", "19", sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=200)
        out = pr.stderr
    except subprocess.TimeoutExpired as e:
        out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
    dump.unlink(missing_ok=True)
    g = lambda pat: (re.findall(pat, out) or [None])[0]  # noqa: E731
    return {"set": s, "problem": p, "gaveup": g(r"LPPROBE_TICKS gave up after (\d+) columns"), "probe": g(r"LP probing: (root LP .*)"),
            "first_stopped": g(r"LP probing: (the first LP stopped: \w+)"), "stopped": "stopped after the presolve" in out}

with ThreadPoolExecutor(3) as ex:
    for row in ex.map(one, [sp for sp in probs if sp[1] not in done]):
        with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
        print(json.dumps(row), flush=True)
