"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、既定 (def) と試作 (all) を rep を変えて解き、res58.jsonl に足す
(analysis/submip_impr_20261106/B.md)。問題ごとに設定の順を入れ替える。ENOMOTO_ の環境変数は消してから設定のものだけを与える
(ENOMOTO_MIP_THREADS=4)。途中から再開できる。使い方: python run58.py <rep> <設定名> [<設定名> ...]"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
ALL = {"ENOMOTO_MIP_SUBB_DIV": "2", "ENOMOTO_MIP_SUBB_POOL": "1", "ENOMOTO_MIP_SUBB_SYM": "1"}
CFGS = {
    "def": {},
    "all": ALL,
    "div": {"ENOMOTO_MIP_SUBB_DIV": "1", "ENOMOTO_MIP_SUBB_POOL": "1"},
    "sym": {"ENOMOTO_MIP_SUBB_SYM": "1"},
}
rep = int(sys.argv[1]); names = sys.argv[2:]
# 書き込み中はリポジトリの外に置く (記録に写すのは終わってから)
OUT = Path("/tmp/claude-0/running/res58.jsonl")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for i, p in enumerate(PROBS):
    for c in (names if (i + rep) % 2 == 0 else names[::-1]):
        if (p, c, rep) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **CFGS[c])
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            res["warn"] = r.stderr.count("MIP: WARNING")
            res["subb"] = [l for l in r.stderr.splitlines() if l.startswith("MIP: SUBB ")]
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "wall": round(time.time() - t0, 1), "result": res}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), res.get("objective"), flush=True)
