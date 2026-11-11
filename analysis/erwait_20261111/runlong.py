"""ERWAIT: 問題を 600 秒・4 スレッドで、設定を続けて回す (reslong.jsonl、ログは logs/ に gzip して残す)。
使い方: python analysis/erwait_20261111/runlong.py ビルド 問題,問題 設定,設定 種,種"""
import gzip, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D)); sys.path.insert(0, str(D.parents[1] / "scripts"))
import run as R  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
build, probs, cfgs, seeds = sys.argv[1], sys.argv[2].split(","), sys.argv[3].split(","), [int(s) for s in sys.argv[4].split(",")]
for s in seeds:
    for p in probs:
        rb.fetch(p, D.parents[1] / ".miplib_cache")
        for c in cfgs:
            R.run(build, c, s, 600, p, out=D / "reslong.jsonl", logdir=D / "logs")
            log = D / "logs" / f"{p}_{build}_{c}_s{s}_600.log"
            if log.exists():
                (D / "logs" / (log.name + ".gz")).write_bytes(gzip.compress(log.read_bytes())); log.unlink()
