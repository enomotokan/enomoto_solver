"""lpbench.py の Mittelmann の問題 (`MITTEL`) を取って .mittelmann_cache/mps/<名前>.mps.gz に置く (取得は scripts/run_mittelmann_benchmark.py の
`ensure_raw`、emps 形式は展開してから gzip にする)。python analysis/lpiter_20261127/lpbench_prep.py [--shard i/n]"""
import gzip, shutil, sys, tempfile
from pathlib import Path
D = Path(__file__).resolve().parent
REPO = D.parents[1]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(D))
import run_mittelmann_benchmark as rm  # noqa: E402
from lpbench import MITTEL  # noqa: E402
si, sn = map(int, (sys.argv[sys.argv.index("--shard") + 1] if "--shard" in sys.argv else "0/1").split("/"))
cache = REPO / ".mittelmann_cache"
(cache / "mps").mkdir(parents=True, exist_ok=True)
rel_of = {n: rel for n, rel, *_ in rm.PROBLEMS}
for i, n in enumerate(MITTEL):
    if i % sn != si:
        continue
    out = cache / "mps" / f"{n}.mps.gz"
    if out.exists():
        continue
    raw = rm.ensure_raw(cache, n, rel_of[n])
    with tempfile.TemporaryDirectory(dir=cache) as wd:
        mps = rm.materialize_mps(cache, rel_of[n], raw, Path(wd))
        with open(mps, "rb") as src, gzip.open(str(out) + ".part", "wb", compresslevel=3) as dst:
            shutil.copyfileobj(src, dst, 1 << 20)
    Path(str(out) + ".part").rename(out)
    raw.unlink()
    print(n, out.stat().st_size >> 20, "MB", flush=True)
