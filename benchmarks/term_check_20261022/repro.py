"""physiciansched6-2 を e2iohleq の環境変数・ENOMOTO_MIP_LOG=1 で繰り返し解き、ログを残す。
使い方: python repro.py OUTDIR N [TL] [extra KEY=VAL ...]"""
import json, sys, os
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "psched_ioeq_20261022"))
import run as R

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
n = int(sys.argv[2]); tl = float(sys.argv[3]) if len(sys.argv) > 3 else 60.0
extra = dict(a.split("=", 1) for a in sys.argv[4:])
mps = R.CACHE / "physiciansched6-2.mps"
for k in range(n):
    if (out / f"rep{k}.json").exists():
        continue
    r, err = R.run(mps, tl, 4, {**R.CONFIGS["e2iohleq"], **extra}, log=True)
    r["rep"] = k; r["extra"] = extra
    (out / f"rep{k}.log").write_text(err)
    (out / f"rep{k}.json").write_text(json.dumps(r))
    print(json.dumps({x: r.get(x) for x in ("rep", "status", "objective", "bound", "root_bound", "nodes", "time")}), flush=True)
