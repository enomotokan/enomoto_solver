"""HNFPRE の数え上げに使う問題の集合 (2026-11-07)。
58 問 (analysis/cbcimpl_20261029/probs.py)、Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json)、
MIPLIB 2017 の実行不能の問題 45 問 (miplib.zib.de の tag_infeasible.html から、miplib_infeasible.txt)、market split 型。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
import probs as p58
import run_miplib_benchmark as rb
P58 = list(p58.PROBS)
MITT = sorted({p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part})
INFEAS = (D / "miplib_infeasible.txt").read_text().split()
MSPLIT = ["markshare1", "markshare2", "markshare_4_0", "markshare_5_0"]
SETS = {"p58": P58, "mitt": MITT, "infeas": INFEAS, "msplit": MSPLIT}
ALL = list(dict.fromkeys(P58 + MITT + INFEAS + MSPLIT))
if __name__ == "__main__":
    ok, bad = 0, []
    for p in ALL:
        try:
            rb.fetch(p, WT / ".miplib_cache"); ok += 1
        except Exception as e:
            bad.append((p, str(e)[:80]))
    print(ok, "fetched; failed:", bad)
