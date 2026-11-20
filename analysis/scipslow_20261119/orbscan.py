"""Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) で、ORBPP (静的なオービトープ) が働く問題を洗い出す。
前処理と対称性の検出の後で止め (ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3 + STOP_PRESOLVED)、ORBPP と ORBPP_REPROBE の行を拾う。1 スレッド・nice 19・1 問 120 秒まで。
python orbscan.py → orbscan.jsonl (途中から再開できる)"""
import json, os, re, subprocess, sys, tempfile
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
probs = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
OUT = Path(os.environ.get("ORBSCAN_OUT", str(D / "orbscan.jsonl")))  # 走っている間は scratchpad に書き、終わってから写す
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
dump = Path(tempfile.gettempdir()) / "orbscan_dump.mps"
for p in probs:
    if p in done: continue
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        row = {"problem": p, "error": str(e)[:200]}
    else:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_ORBPP_REPROBE": "1",
                    "ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3": str(dump), "ENOMOTO_MIP_DEBUG_STOP_PRESOLVED": "1"})
        try:
            pr = subprocess.run(["nice", "-n", "19", sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "1"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=200)
            out = pr.stderr
        except subprocess.TimeoutExpired as e:
            out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
        orb = re.findall(r"MIP: symmetry: (\d+) full orbitopes \(([^)]*)\)", out)
        pp = re.findall(r"ORBPP (\d+) of (\d+) orbitopes static.*fixed (\d+) columns", out)
        rep = re.findall(r"ORBPP_REPROBE: .*", out)
        row = {"problem": p, "orbitopes": orb[:1], "orbpp": pp[:1], "reprobe": rep[:1], "stopped": "stopped after the presolve" in out}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)
