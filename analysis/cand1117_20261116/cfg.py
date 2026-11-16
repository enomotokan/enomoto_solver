"""束 cand1117 (2026-11-16、案)。cand1116 (analysis/cand1116_20261115/cfg.py) に、
  - ENOMOTO_MIP_TICKS=ntwumcflshk + ENOMOTO_MIP_DETHASH=1 (timedep で採用済み、analysis/timedep_20261114.md 12 節。6e875bdf7 以降の DETHASH の直しを含む版)
  - ENOMOTO_MIP_DFSBB=2 (HiGHS と同じ木の進め方: 潜りは深さ優先で 100 ノードで打ち切り、待ち行列は 10 回に 1 回下界最小、ほかは 0.5 下界 + 0.5 推定値。
    analysis/treeclose_20261114.md 9.40・9.44 節。b58v の 58 問 × 種 1〜4 で −5.3% [−9.6, −1.3]・解けた 178/175・食い違い 0)
を足したもの。c19fc41d9 以降の版で回す。比べる相手は cand1116 と前の基準 cand1114、Mittelmann では HiGHS 1.15.1 も。"""
import importlib.util
from pathlib import Path

def _load(rel, name):
    sp = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parent.parent / rel)
    m = importlib.util.module_from_spec(sp)
    sp.loader.exec_module(m)
    return m

_c16 = _load("cand1116_20261115/cfg.py", "cfg1116")
CAND1114 = _c16.CAND1114
CAND1116 = _c16.CAND1116
CAND1117 = {**CAND1116, "ENOMOTO_MIP_TICKS": "ntwumcflshk", "ENOMOTO_MIP_DETHASH": "1", "ENOMOTO_MIP_DFSBB": "2"}
CFGS = {"cand1114": CAND1114, "cand1116": CAND1116, "cand1117": CAND1117, "highs": {}}

# 切り分け I (10teams・neos-827175 が cand1114 より全種で遅い理由、planI.py): cand1117 から 1 つずつ外した設定と、cand1116 に 1 つずつ足した設定
_DIFF16 = [k for k in sorted(set(CAND1114) | set(CAND1116)) if CAND1114.get(k) != CAND1116.get(k)]  # cand1114 → cand1116 で変わった鍵
_ADD17 = ["ENOMOTO_MIP_TICKS", "ENOMOTO_MIP_DETHASH", "ENOMOTO_MIP_DFSBB"]                         # cand1116 → cand1117 で足した鍵
def _minus(k):
    d = dict(CAND1117)
    if k in CAND1114: d[k] = CAND1114[k]
    else: d.pop(k, None)
    return d
ISO = {}
for _k in _DIFF16 + _ADD17:
    ISO["m_" + _k.replace("ENOMOTO_MIP_", "").lower()] = _minus(_k)          # cand1117 から _k を外す (cand1114 の値に戻す)
for _k in _ADD17:
    ISO["c16+" + _k.replace("ENOMOTO_MIP_", "").lower()] = {**CAND1116, _k: CAND1117[_k]}  # cand1116 に _k だけ足す
# (cand1116 + TICKS + DETHASH は m_dfsbb と同じ)
CFGS.update(ISO)

# 誤答の切り分け W (unitcal_7 種 2 で cand1116 が最適値を切り落とした、planW.py): cand1116 から cand1114 → cand1116 で変わった鍵を 1 つずつ外す
def _minus16(k):
    d = dict(CAND1116)
    if k in CAND1114: d[k] = CAND1114[k]
    else: d.pop(k, None)
    return d
WRONG = {"w16_m_" + _k.replace("ENOMOTO_MIP_", "").lower(): _minus16(_k) for _k in _DIFF16}
CFGS.update(WRONG)
