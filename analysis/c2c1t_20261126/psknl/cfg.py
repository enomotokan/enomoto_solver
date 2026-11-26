"""PSKIP の P での確かめ (analysis/c2c1t_20261126 の 10 節、2026-11-26、親の依頼) の設定。
**この台本の `def_t1`・`def_t4` は既定ではなく c2c (analysis/combo2_20261125/cfg.py の C2C、nra を r0k23p にした形)**。
summ.py は名前が def_ の設定を比べる相手にするので、c2c をその名前に当てた。試作 `nl` は c2c + ENOMOTO_MIP_SB_PROP_NO_LP=1 (強分枝の子の伝播は子を切る判定とノードに置く境界にだけ使い、子の LP には渡さない、README 10 節)。"""
import importlib.util as _iu
from pathlib import Path as _P
_s = _iu.spec_from_file_location("combo2_cfg", _P(__file__).resolve().parents[2] / "combo2_20261125" / "cfg.py")
_m = _iu.module_from_spec(_s); _s.loader.exec_module(_m)
_C2C = _m.C2C
C2CP = dict(_C2C, ENOMOTO_MIP_NODEROWS_ADAPT="r0k23p")
PK = dict(C2CP, ENOMOTO_MIP_SB_PROP_NO_LP="1")
P1 = {1: ("nl", PK, True, True, "c2c1t_20261126")}
CFGS = {"def_t1": C2CP, "def_t4": C2CP, "nl_t1": PK, "nl_t4": PK}
