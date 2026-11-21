"""直す前と直した後で pe の求解が同じ道になるかを確かめる (2026-11-21、README 5 節)。
python equiv.py 直す前の版を入れた dir [問題 ...]: 各問題を 1 スレッド・種 1・pe・ENOMOTO_MIP_TIMEDEP=b (決定的な設定、analysis/det1t_20261120)・
ノード数の上限 NL で、直す前 (PYTHONPATH に dir を足す) と直した後 (入っている版) で 1 回ずつ解き、結果と、ログの統計の行 (回数だけで時間を伏せる) を
比べて equiv.jsonl に足す。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OLD = sys.argv[1]
PS = sys.argv[2:] or ["supportcase26", "30n20b8", "timtab1", "neos5", "gen-ip054", "neos-1582420", "rococoC10-001000"]
NL, TL = 5000, 1800
OUT = D / "equiv.jsonl"
KEYS = ("conflict analysis (HiGHS style)", "dual proofs and conflicts", "literal conflicts:", "cut pool propagation: calls",
        "conflict levels (all threads)", "conflict analyses (all threads)", "node diag", "branching:")


def stats(err):
    out = {}
    for l in err.splitlines():
        for k in KEYS:
            if k in l and "sub-MIP" not in l:
                # 時間 (秒) と時刻を伏せる
                out[k] = re.sub(r"[\d.]+s\b", "#s", re.sub(r"(LP secs|node cut secs) [\d.]+", r"\1 #", l))
    return out


for P in PS:
    mps = rb.fetch(P, WT / ".miplib_cache")
    rec = {"problem": P, "nl": NL}
    for tag in ("old", "new"):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN="1", ENOMOTO_MIP_TIMEDEP="b", ENOMOTO_MIP_NODE_LIMIT=str(NL),
                   ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_CONFLICT_LEV_DIAG="1")
        if tag == "old":
            env["PYTHONPATH"] = OLD + os.pathsep + env.get("PYTHONPATH", "")
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "1"],
                           cwd=WT, env=env, capture_output=True, text=True)
        try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
        rec[tag] = {"result": res, "stats": stats(r.stderr)}
    o, n = rec["old"], rec["new"]
    rec["same"] = (o["result"].get("status"), o["result"].get("nodes"), o["result"].get("objective"), o["result"].get("bound"), o["stats"]) == \
                  (n["result"].get("status"), n["result"].get("nodes"), n["result"].get("objective"), n["result"].get("bound"), n["stats"])
    with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
    print(P, rec["same"], o["result"].get("status"), o["result"].get("nodes"), o["result"].get("time"), n["result"].get("nodes"), n["result"].get("time"), flush=True)
