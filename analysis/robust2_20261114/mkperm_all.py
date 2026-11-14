"""58 問の並べ替え 1〜3 の MPS をまとめて作る (計測の前に、並べた計測が同じファイルを同時に書かないように)"""
import sys
from pathlib import Path
from concurrent.futures import ProcessPoolExecutor
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029")); sys.path.insert(0, str(WT / "analysis/robust_20261114"))
from probs import PROBS, rb  # noqa: E402
import perm as pm  # noqa: E402
def one(a):
    p, k = a
    out = WT / ".miplib_cache/perm" / f"{p}_p{k}.mps"
    if not out.exists(): out.parent.mkdir(exist_ok=True); pm.make(rb.fetch(p, WT / ".miplib_cache"), k, out)
    return p, k
if __name__ == "__main__":
    with ProcessPoolExecutor(4) as ex:
        for r in ex.map(one, [(p, k) for p in PROBS for k in (1, 2, 3)]): pass
    print("ok")
