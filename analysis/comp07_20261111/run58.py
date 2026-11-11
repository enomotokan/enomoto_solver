"""COMP07: 58 問 (analysis/restchain_20261111/jobs58.txt と同じ問題) を、同じビルドの設定どうしで交互に 60 秒・4 スレッドで解き res58.jsonl に足す
(問題ごとに設定を続けて回す。途中から再開できる)。ログは logs58/ (gzip しない、コミットしない)。
使い方: python analysis/comp07_20261111/run58.py ビルド 設定,設定 種,種 [分担の数 分担の番号]
分担のときは出力を res58_sh<番号>.jsonl に分ける (分担セッションの結果をまとめてコミットしても衝突しない)。"""
import os, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D)); sys.path.insert(0, str(D.parents[1] / "scripts"))
import run as R  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
PROBS = sorted({l.split()[1] for l in open(D.parent / "restchain_20261111" / "jobs58.txt") if l.strip() and not l.startswith("#")})
assert len(PROBS) == 58
build, cfgs, seeds = sys.argv[1], sys.argv[2].split(","), [int(s) for s in sys.argv[3].split(",")]
nsh, ish = (int(sys.argv[4]), int(sys.argv[5])) if len(sys.argv) > 5 else (1, 0)
for s in seeds:
    for i, p in enumerate(PROBS):
        if i % nsh != ish:
            continue
        rb.fetch(p, D.parents[1] / ".miplib_cache")
        for c in cfgs:
            R.run(build, c, s, 60, p, out=D / ("res58.jsonl" if nsh == 1 else f"res58{os.environ.get('COMP07_TAG', '')}_sh{ish}.jsonl"), logdir=D / "logs58")
