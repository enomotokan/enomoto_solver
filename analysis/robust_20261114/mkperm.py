"""58 問の並べ替え 1〜K を .miplib_cache/perm/<問題>_p<k>.mps に作る (有れば飛ばす): python mkperm.py [K=3]"""
import sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029")); sys.path.insert(0, str(D))
from probs import PROBS, rb  # noqa: E402
import perm  # noqa: E402
K = int(sys.argv[1]) if len(sys.argv) > 1 else 3
for p in PROBS:
    src = rb.fetch(p, WT / ".miplib_cache")
    for k in range(1, K + 1):
        out = WT / ".miplib_cache/perm" / f"{p}_p{k}.mps"
        if out.exists(): continue
        t = time.time(); perm.make(src, k, out); print(p, k, round(time.time() - t, 1), flush=True)
