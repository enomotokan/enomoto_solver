"""計測する問題: 負けている 21 問 (benchmarks/eprostop_loss21_16t_20261028.jsonl の問題) + MIPLIB 40 問 (重複 3 問を除き 58 問)。"""
import sys
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb
LOSS = ['drayage-100-23', 'drayage-25-23', 'exp-1-500-5-5', 'h80x6320d', 'hypothyroid-k1', 'mc11', 'mzzv42z',
        'neos-3083819-nubu', 'neos-827175', 'neos-860300', 'neos-911970', 'neos-960392', 'neos17', 'neos8', 'ns1116954',
        'nu25-pr12', 'p200x1188c', 'physiciansched6-2', 'roll3000', 'sp150x300d', 'swath1']
PROBS = LOSS + [p for p in rb.PROBLEMS if p not in LOSS]
SETS = {"loss21": LOSS, "miplib40": rb.PROBLEMS}
if __name__ == "__main__":
    for p in PROBS:
        print(p, rb.fetch(p, WT / ".miplib_cache"))
