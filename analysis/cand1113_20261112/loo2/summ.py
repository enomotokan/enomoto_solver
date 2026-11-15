"""res_L_s*.jsonl を表にする (../loo/summ.py と同じ形、問題だけ違う)。"""
import runpy, sys
from pathlib import Path
D = Path(__file__).resolve().parent
src = (D.parent / "loo" / "summ.py").read_text().replace(
    'P = ["neos-960392", "binkar10_1", "roll3000", "pk1", "10teams", "neos8", "h80x6320d"]',
    'P = ["enlight_hard", "neos-1122047", "cbs-cta"]')
sys.path.insert(0, str(D))
exec(compile(src.replace('Path(__file__).resolve().parent', 'Path(%r)' % str(D)), "summ", "exec"))
