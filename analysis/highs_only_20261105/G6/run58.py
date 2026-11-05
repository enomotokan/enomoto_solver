"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、既定 (def) と試作を 1 回ずつ解き、res58.jsonl に足す。
問題ごとに設定の順を入れ替える。ENOMOTO_ の環境変数は消してから設定のものだけを与える (ENOMOTO_MIP_THREADS=4)。途中から再開できる。
使い方: python run58.py <設定名> [<設定名> ...]  (設定名は CFGS のキー)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CFGS = {
    "def": {},
    "ho6": {"ENOMOTO_MIP_HO6_STALL": "1", "ENOMOTO_MIP_HO6_SYMWORK": "1", "ENOMOTO_MIP_HO6_SYMREFINE": "1",
            "ENOMOTO_T_MIP_PRESOLVE_WORK_SYM": "5e8", "ENOMOTO_T_MIP_PRESOLVE_WORK_SYM_LOG": "1e9", "ENOMOTO_T_MIP_SYM_TIME": "1", "ENOMOTO_MIP_CUTSEL_PROBE": "gain"},
}
names = sys.argv[1:]
OUT = D / "res58.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"]))
for i, p in enumerate(PROBS):
    for c in (names if i % 2 == 0 else names[::-1]):
        if (p, c) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", **CFGS[c])
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            res["warn"] = r.stderr.count("MIP: WARNING")
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "wall": round(time.time() - t0, 1), "result": res}) + "\n")
