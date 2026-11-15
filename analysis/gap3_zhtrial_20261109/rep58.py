"""58 問 (4 スレッド・60 秒、種 1・2) の比べ。使い方: python rep58.py p58.jsonl 試作の設定 [...] > p58_report.txt"""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import benchlib as bl  # noqa: E402
R = bl.table(bl.load([sys.argv[1]]))
for b in sys.argv[2:]:
    keys = sorted(k for k in R if "def" in R[k] and b in R[k])
    bl.report(R, "def", b, keys, label=b)
