"""58 問 (tree_restart_guard、analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で設定ごとに 1 回ずつ解き、S2/bench58.jsonl に足す (2026-11-06、S2)。
使い方: python bench58_s2.py <設定名,...> <回> (種 ENOMOTO_T_MIP_SEED = 回。設定の順は回ごとに逆、ソルバーは同時に 1 つ。途中から再開できる)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CFGS = {
    "def": {},
    "inc": {"ENOMOTO_MIP_TREE_RESTART": "inc"},
    "force": {"ENOMOTO_MIP_TREE_RESTART": "force"},
    "forcekc": {"ENOMOTO_MIP_TREE_RESTART": "force", "ENOMOTO_MIP_TREE_RESTART_GUARD": "kc"},
    "forcekC": {"ENOMOTO_MIP_TREE_RESTART": "force", "ENOMOTO_MIP_TREE_RESTART_GUARD": "kC"},
    "forceKR": {"ENOMOTO_MIP_TREE_RESTART": "force", "ENOMOTO_MIP_TREE_RESTART_GUARD": "KR"},
    "incC": {"ENOMOTO_MIP_TREE_RESTART": "inc", "ENOMOTO_MIP_TREE_RESTART_GUARD": "C"},
    "incc": {"ENOMOTO_MIP_TREE_RESTART": "inc", "ENOMOTO_MIP_TREE_RESTART_GUARD": "c"},
}
OUT = D / os.environ.get("BENCH58_OUT", "bench58.jsonl")  # whl5 の回は bench58_w5.jsonl
cfgs, rep = sys.argv[1].split(","), int(sys.argv[2])
if rep % 2:
    cfgs = cfgs[::-1]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for p in PROBS:
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[c])
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), flush=True)
