"""根の LP が重い問題 (1 節の比べ)。python probs.py で取ってくる。"""
import sys
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "scripts"))
HEAVY = ['rail01', 'momentum1', 'satellites2-40', 'uccase12', 'uccase9', 'supportcase42', 'neos-873061', 'opm2-z10-s4',
         'thor50dday', 'sing326', 'sing44', 'snp-02-004-104', 'neos-3216931-puriri', 'satellites2-60-fs',
         'rail507', 's250r10', 'physiciansched3-3', 'square41', 'savsched1', 'neos-5052403-cygnet']
if __name__ == "__main__":
    import run_miplib_benchmark as rb
    for p in HEAVY:
        try:
            print(p, rb.fetch(p, WT / ".miplib_cache"), flush=True)
        except Exception as e:  # noqa: BLE001
            print(p, "FAIL", e, flush=True)
