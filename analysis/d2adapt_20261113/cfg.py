"""D2 (analysis/highsloss2_20261113.md の D2a・D2b) の設定。土台は cand1114 (analysis/cand1114_20261113/cfg.py の `CAND1114`)。
- base: cand1114 だけ
- d2a: + `ENOMOTO_MIP_HL2_D2=a` (ノードの分離の前の効いていないカットの一括削除で、待ち行列のノードの保存した基底で効いているカットを守る。
  年齢 30 以下・max(0.5 × 元の行, 50) 本まで)
- d2b: + `ENOMOTO_MIP_HL2_D2=b` (取り出したノードの保存した基底で効いていたカットが LP から消えていれば足し直して保存した基底を置く)
- d2ab: 両方
- d2B: 適応版 (ENOMOTO_MIP_HL2_D2=B)
- d2br・d2Br: + r (足し直した行が効かなくなったら年齢 5 で外す)"""
import importlib.util
from pathlib import Path
_spec = importlib.util.spec_from_file_location("cand1114_cfg", Path(__file__).resolve().parents[1] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_m)
CAND1114 = _m.CAND1114
CFGS = {
    "base": dict(CAND1114),
    "d2a": dict(CAND1114, ENOMOTO_MIP_HL2_D2="a"),
    "d2b": dict(CAND1114, ENOMOTO_MIP_HL2_D2="b"),
    "d2ab": dict(CAND1114, ENOMOTO_MIP_HL2_D2="ab"),
    "d2B": dict(CAND1114, ENOMOTO_MIP_HL2_D2="B"),
    "d2br": dict(CAND1114, ENOMOTO_MIP_HL2_D2="br"),
    "d2Br": dict(CAND1114, ENOMOTO_MIP_HL2_D2="Br"),
}
