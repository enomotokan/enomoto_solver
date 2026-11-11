"""COMP07: 指定の問題を、指定のビルドの設定どうしで種ごとに交互に解き res58.jsonl に足す (mc11・noswot の害の確かめ用)。
使い方: python analysis/comp07_20261111/run_probs.py 秒 ビルド 問題,問題 設定,設定 種,種"""
import sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
import run as R  # noqa: E402
tl, b, probs, cfgs, seeds = int(sys.argv[1]), sys.argv[2], sys.argv[3].split(","), sys.argv[4].split(","), [int(s) for s in sys.argv[5].split(",")]
for s in seeds:
    for p in probs:
        for c in cfgs:
            R.run(b, c, s, tl, p, out=D / "res58.jsonl", logdir=D / "logs58")
