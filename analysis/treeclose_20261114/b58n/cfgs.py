"""b58n の 3 つの設定 (analysis/treeclose_20261114.md 9.25 節、利用者の指示: =5 の区間を短くして観測を増やす)。
どれも束 cand1116 (analysis/cand1116_20261115/cfg.py の CAND1116) の上の `=5` EI。区間の長さ (主のスレッドの LP の反復) だけを変え、
観測の窓は区間に合わせて広げ、割引は経過した反復 2 万ごとに 1.05 倍 (`NB5_DECAY_WORK=20000`、区間の数ではなく反復で測る) に揃える。
尺度・雑音の選び直し (周辺尤度の格子 54 通り) は反復 2 万ごとに 1 回 (`NB5_REFIT`) に揃える (区間 2000 で毎回選び直すと choose が主のスレッドの約 9% を食うため)。
w20 = 区間 20000・窓 40 (b58l の nb5e と同じ区間・窓。割引だけ反復で測る形)、w5 = 区間 5000・窓 100、w2 = 区間 2000・窓 150。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1116, ENOMOTO_MIP_NODESEL_BANDIT="5", ENOMOTO_MIP_NB5_ACQ="ei", ENOMOTO_T_MIP_NB5_DECAY_WORK="20000")
CFGS = {
    "w20": dict(BASE, ENOMOTO_T_MIP_NB_WORK="20000", ENOMOTO_T_MIP_NB5_WINDOW="40", ENOMOTO_T_MIP_NB5_REFIT="1"),
    "w5": dict(BASE, ENOMOTO_T_MIP_NB_WORK="5000", ENOMOTO_T_MIP_NB5_WINDOW="100", ENOMOTO_T_MIP_NB5_REFIT="4"),
    "w2": dict(BASE, ENOMOTO_T_MIP_NB_WORK="2000", ENOMOTO_T_MIP_NB5_WINDOW="150", ENOMOTO_T_MIP_NB5_REFIT="10"),
}
