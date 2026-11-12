"""ERWAIT: 58 問 (analysis/restchain_20261111/jobs58.txt と同じ問題) を、同じビルドの設定どうしで問題ごとに続けて 60 秒・4 スレッドで解き
res58.jsonl (分担なら res58_sh<番号>.jsonl) に足す。途中から再開できる。ログは logs58/ (コミットしない)。
使い方: python analysis/erwait_20261111/run58.py ビルド 設定,設定 種,種 [分担の数 分担の番号 [結果の接頭辞 (既定 res58)]]"""
import sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D)); sys.path.insert(0, str(D.parents[1] / "scripts"))
import run as R  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
PROBS = sorted({l.split()[1] for l in open(D.parent / "restchain_20261111" / "jobs58.txt") if l.strip() and not l.startswith("#")})
assert len(PROBS) == 58
build, cfgs, seeds = sys.argv[1], sys.argv[2].split(","), [int(s) for s in sys.argv[3].split(",")]
nsh, ish = (int(sys.argv[4]), int(sys.argv[5])) if len(sys.argv) > 5 else (1, 0)
pre = sys.argv[6] if len(sys.argv) > 6 else "res58"
for s in seeds:
    for i, p in enumerate(PROBS):
        if i % nsh != ish:
            continue
        rb.fetch(p, D.parents[1] / ".miplib_cache")
        for c in cfgs:
            R.run(build, c, s, 60, p, out=D / (f"{pre}.jsonl" if nsh == 1 else f"{pre}_sh{ish}.jsonl"), logdir=D / "logs58")
