"""束 cand1115 (2026-11-15、案)。cand1114 (analysis/cand1114_20261113/cfg.py) に足す環境変数。
足すもの (理由は各記録):
  OBJPROBE=rpc (analysis/objprobe_20261113)、TREE2_INTAGG=d (analysis/presolvegap_20261113.md)、
  ROOT_MAX_CUTS=150 (rmc150、analysis/review2_20261113.md)、POOL_TAB=x (analysis/dualeq2_20261113.md §6〜8)、
  TIMEDEP Et2 (analysis/timedep_20261114/final_cfg.py、8 節)、ROBUST=pgq (analysis/robust_20261114.md 6.4 節)、
  RHEUR=es (analysis/rootheur_20261114.md §4。追加の種の確かめは省き、束の計測で見る)。
保留: TC=a (analysis/treeclose_20261114.md 6 章、600 秒の 15 問 × 種 2・3 の結果待ち)。
入れないもの: CLQPRB (puriri)、ORDINV・PERMRACE・RSTROB (robust2)、gk、ROBUST の s・f、TC=r/f、cpp・tcv。
正しさの修正 40e80904・f1c14c30・dfb8186d・9b839dd1・7d5e2b21・7917d993 は既定に入っているので両側に入る。"""
import importlib.util
from pathlib import Path

def _load(rel, name):
    sp = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parent.parent / rel)
    m = importlib.util.module_from_spec(sp)
    sp.loader.exec_module(m)
    return m

CAND1114 = _load("cand1114_20261113/cfg.py", "cfg1114").CAND1114
TIMEDEP_FINAL = _load("timedep_20261114/final_cfg.py", "timedep_final").TIMEDEP_FINAL

ADD1115 = {
    "ENOMOTO_MIP_OBJPROBE": "rpc",
    "ENOMOTO_MIP_TREE2_INTAGG": "d",
    "ENOMOTO_T_MIP_ROOT_MAX_CUTS": "150",
    "ENOMOTO_MIP_POOL_TAB": "x",
    **TIMEDEP_FINAL,
    "ENOMOTO_MIP_ROBUST": "pgq",
    "ENOMOTO_MIP_RHEUR": "es",
}
CAND1115 = {**CAND1114, **ADD1115}
CFGS = {"cand1114": CAND1114, "cand1115": CAND1115, "highs": {}}
