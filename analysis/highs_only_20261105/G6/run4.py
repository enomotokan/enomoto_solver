"""対称性の検出が変わる 4 問を、既定 (def) と対称性の試作 (sym) で 60 秒・3 回ずつ (交互) 解き、res4.jsonl に足す。
ログ (ENOMOTO_MIP_LOG=1) は logs4/<問題>_<設定>_<回>.log.gz。途中から再開できる。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
SYM = {"ENOMOTO_MIP_HO6_SYMWORK": "1", "ENOMOTO_MIP_HO6_SYMREFINE": "1", "ENOMOTO_T_MIP_PRESOLVE_WORK_SYM": "5e8",
       "ENOMOTO_T_MIP_PRESOLVE_WORK_SYM_LOG": "1e9", "ENOMOTO_T_MIP_SYM_TIME": "1"}
CFGS = {"def": {}, "sym": SYM}
PROBS = ["neos-827175", "neos-960392", "neos8", "physiciansched6-2"]
OUT = D / "res4.jsonl"; (D / "logs4").mkdir(exist_ok=True)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for rep in range(3):
    for p in PROBS:
        for c in (["def", "sym"] if rep % 2 == 0 else ["sym", "def"]):
            if (p, c, rep) in done:
                continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **CFGS[c])
            t0 = time.time()
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            with gzip.open(D / "logs4" / f"{p}_{c}_{rep}.log.gz", "wt") as f:
                f.write(r.stderr)
            try:
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:200]}
            res["warn"] = r.stderr.count("MIP: WARNING")
            with open(OUT, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "wall": round(time.time() - t0, 1), "result": res}) + "\n")
