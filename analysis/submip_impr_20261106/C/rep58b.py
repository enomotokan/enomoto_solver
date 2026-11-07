"""bench58b.jsonl (既定と SUBFIX=rtcqf、58 問・60 秒・種 1・2) を benchlib の report で比べる。使い方: python rep58b.py"""
import sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[1]))
import benchlib as bl  # noqa: E402
R = bl.table(bl.load([here / "bench58b.jsonl"]))
keys = list(R)
bl.report(R, "def", "rtcqf", keys, noise=bl.load_noise())
