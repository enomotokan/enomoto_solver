"""LP face のサブ MIP が働く (固定率 ≥ 0.3) 問題を数える (走査)。試作 ENOMOTO_MIP_LPFACE=cf で、
P (MIPLIB 58 問) と M (Mittelmann 186 問、P と重ならないもの) を 4 スレッド・種 1・TL 秒で回し、ログの「LP face」の行を拾う。
python lfscan.py [TL] → LFSCAN_OUT (既定 lfscan.jsonl、途中から再開できる)"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = sorted({x[0] for s in json.loads((WT / "analysis/scipslow_20261119/shards_P.json").read_text())["shards"] for x in s})
M = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P]
OUT = Path(os.environ.get("LFSCAN_OUT", str(D / "lfscan.jsonl")))
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
TL = float(sys.argv[1]) if len(sys.argv) > 1 else 60
for s, p in probs:
    if p in done: continue
    row = {"set": s, "problem": p}
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        row["error"] = str(e)[:200]
    else:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_LPFACE": "cf"})
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 300)
            out = pr.stderr
            try:
                r = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
                row.update({k: r.get(k) for k in ("status", "objective", "root_bound", "time", "first_solution_time")})
            except Exception: row["status"] = "crash"  # noqa: BLE001
        except subprocess.TimeoutExpired as e:
            out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""); row["status"] = "timeout"
        m = re.findall(r"LP face \(cf\): (\d+) of (\d+) integer columns fixed, objective row <= (\S+) \(LP (\S+)\), status (.*), found (\w+)", out)
        if m:
            a, b, zb, z, st, fd = m[-1]
            row.update({"face": "ran", "fixed": int(a), "nint": int(b), "zbar": float(zb), "status_sub": st, "found": fd == "true"})
        m = re.findall(r"LP face: only (\d+) of (\d+) integer columns fixed", out)
        if m and "face" not in row:
            row.update({"face": "low", "fixed": int(m[-1][0]), "nint": int(m[-1][1])})
        t = re.findall(r"root heuristic LP face: (\w+) \(([\d.]+)s\)", out)
        if t: row["face_secs"] = float(t[-1][1])
        if "face" not in row: row["face"] = "none"
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)
