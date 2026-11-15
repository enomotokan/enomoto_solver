"""58 問と Mittelmann 186 問の MPS を .miplib_cache に取る (並列 4)。"""
import json, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb
from probs import PROBS
mitt = sorted({p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part})
def f(p):
    try: rb.fetch(p, WT / ".miplib_cache"); return p, "ok"
    except Exception as e: return p, f"err {e}"
with ThreadPoolExecutor(4) as ex:
    for p, s in ex.map(f, list(dict.fromkeys(PROBS + mitt))):
        print(p, s, flush=True)
