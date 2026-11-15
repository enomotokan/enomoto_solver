"""束 cand1116 (2026-11-15、案)。cand1115 の組み直し (analysis/cand1115_20261115/cd_kekka.md) の後の束。
cand1116 = cand1115d (cand1115 から TIMEDEP Et2 と rmc150 を外したもの) に、
  - TC を b → bs (引き継ぐ固定を親の鎖で共有する。fast0507 で最大 RSS 4.2 GB → 2.7 GB、mem_fast0507.md。引き継ぐ固定そのものは b と同じ)
  - ENOMOTO_MIP_SUBMIP_HALT=1 (別スレッドのサブ MIP の止めの合図を LP・内点法などにも届ける。analysis/timedep_20261114.md 9 節。
    h58 で −2.0% [−7.5, +3.0]、tbfp-network は合図から 508 秒 → 0.02 秒で止まる。c923b015 以降の版が要る)
  を足したもの。
cand1116f = cand1116 + 深さ優先寄り (analysis/treeclose_20261114.md 9 章、dfs58 で −2.8% [−8.0, +3.2]。mcsched が閉じる)。
比べる相手は cand1114 (0.1.7 の既定と同じ)。"""
import importlib.util
from pathlib import Path

def _load(rel, name):
    sp = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parent.parent / rel)
    m = importlib.util.module_from_spec(sp)
    sp.loader.exec_module(m)
    return m

_c15 = _load("cand1115_20261115/cfg.py", "cfg1115")
CAND1114 = _c15.CAND1114
CAND1116 = {**_c15.CAND1115D, "ENOMOTO_MIP_TC": "bs", "ENOMOTO_MIP_SUBMIP_HALT": "1"}
DFS = {"ENOMOTO_T_MIP_PLUNGE_QUOT": "1", "ENOMOTO_T_MIP_SIBLING_QUOT": "1",
       "ENOMOTO_T_MIP_PLUNGE_NODES_MAX": "1000000000", "ENOMOTO_T_MIP_PLUNGE_NODES_DIV": "1"}
CAND1116F = {**CAND1116, **DFS}
CFGS = {"cand1114": CAND1114, "cand1116": CAND1116, "cand1116f": CAND1116F, "highs": {}}
