"""COMP07: (ビルド, 設定, 種) の組を順に回す。種は外側で回し、同じ種の中で組を交互に並べる (時期のずれを対で消す)。
使い方: python analysis/comp07_20261111/run_many.py 秒 種1,種2,... ビルド:設定 ビルド:設定 ..."""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from run import run  # noqa: E402
tl, seeds, pairs = int(sys.argv[1]), [int(s) for s in sys.argv[2].split(",")], [p.split(":") for p in sys.argv[3:]]
for s in seeds:
    for b, c in pairs:
        run(b, c, s, tl)
