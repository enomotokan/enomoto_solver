"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、既定と試作の設定で 1 回ずつ解き、res58.jsonl に書く。
問題ごとに設定の順を入れ替える。使い方: python analysis/tree_close1_20261104/run58.py 設定名..."""
import json, os, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
CFGS = {
    "def": {},
    "trp": {"ENOMOTO_MIP_TREE1_RESTART": "1", "ENOMOTO_MIP_TREE1_POOL": "1", "ENOMOTO_MIP_RESTART_PC": "1"},
    "tp": {"ENOMOTO_MIP_TREE1_POOL": "1"},
}
names = sys.argv[1:] or ["def", "trp"]
OUT = D / "res58.jsonl"
base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
for i, p in enumerate(PROBS):
    mps = rb.fetch(p, WT / ".miplib_cache")
    order = names if i % 2 == 0 else names[::-1]
    for c in order:
        os.environ.clear(); os.environ.update(base); os.environ.update(CFGS[c])
        t0 = time.time()
        r = rb.run_worker("ours", mps, 60, 4)
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "result": r, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, r.get("status"), r.get("time"), r.get("objective"), flush=True)
