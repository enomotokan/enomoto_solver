"""OBJPROBE (2026-11-13) の計測の設定。土台は cand1114 (analysis/cand1114_20261113/cfg.py の CAND1114)。
C1 = 根のカットのループの中の目的の上限つきプロービング (`p`) と被約費用の固定 (`c`)、C2 = 各ラウンドの乱択丸め (`r`)。"""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "cand1114_20261113"))
from cfg import CAND1114  # noqa: E402
CFGS = {
    "base": dict(CAND1114),
    "c1": dict(CAND1114, ENOMOTO_MIP_OBJPROBE="pc"),
    "c2": dict(CAND1114, ENOMOTO_MIP_OBJPROBE="r"),
    "c12": dict(CAND1114, ENOMOTO_MIP_OBJPROBE="rpc"),
}
