"""OBJPROBE (2026-11-13) の計測の設定。土台は cand1114 (analysis/cand1114_20261113/cfg.py の CAND1114)。
C1 = 根のカットのループの中の目的の上限つきプロービング (`p`) と被約費用の固定 (`c`)、C2 = 各ラウンドの乱択丸め (`r`)。"""
import importlib.util
from pathlib import Path
# 同じ名前 (cfg) の自分自身を読まないよう、cand1114 の cfg.py はファイルの場所から読む
_spec = importlib.util.spec_from_file_location("cfg_cand1114", Path(__file__).resolve().parents[1] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_m)
CAND1114 = _m.CAND1114
CFGS = {
    "base": dict(CAND1114),
    "c1": dict(CAND1114, ENOMOTO_MIP_OBJPROBE="pc"),
    "c2": dict(CAND1114, ENOMOTO_MIP_OBJPROBE="r"),
    "c12": dict(CAND1114, ENOMOTO_MIP_OBJPROBE="rpc"),
    "c12h": dict(CAND1114, ENOMOTO_MIP_OBJPROBE="rpch"),   # h = 軽いヒューリスティクスの束 (追加の試作)
}
