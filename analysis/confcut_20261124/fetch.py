"""計測に使う問題ファイルを .miplib_cache に取る: python analysis/confcut_20261124/fetch.py"""
import sys
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
for p in PROBS + ["neos-911970"]:
    try:
        print(p, rb.fetch(p, WT / ".miplib_cache"), flush=True)
    except Exception as e:  # noqa: BLE001
        print(p, "失敗", e, flush=True)
