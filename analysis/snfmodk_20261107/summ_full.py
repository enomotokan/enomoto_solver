"""58 問 (4 スレッド・60 秒) の害の比べ: python summ_full.py [入力.jsonl] [A] [B]"""
import sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0]))
import benchlib as bl  # noqa: E402
f = sys.argv[1] if len(sys.argv) > 1 else "full58.jsonl"
A = sys.argv[2] if len(sys.argv) > 2 else "def"
B = sys.argv[3] if len(sys.argv) > 3 else "snf"
R = bl.table(bl.load([here / f]))
keys = list(R)
bl.report(R, A, B, keys, noise=bl.load_noise(), label="58 問")
for rep in sorted({k[1] for k in keys}):
    bl.report(R, A, B, [k for k in keys if k[1] == rep], label=f"回 {rep}", show_root=False)
