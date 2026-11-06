"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で plain (環境変数なし) と plainlive (ENOMOTO_MIP_NODECUT_LIVE=1) で回す。
回ごとに設定の順を逆にする。使い方: python run58.py 出力.jsonl rep [rep ...]"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[1] / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
out = sys.argv[1]
done = set()
if Path(out).exists():
    for l in open(out):
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for rep in map(int, sys.argv[2:]):
    order = ["plain", "plainlive"] if rep % 2 == 0 else ["plainlive", "plain"]
    for p in PROBS:
        for c in order:
            if (p, c, rep) not in done:
                subprocess.run([sys.executable, str(D / "run1.py"), out, c, p, "60", str(rep)])
