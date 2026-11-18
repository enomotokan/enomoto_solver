"""SEP_FAST の対の比べの設定: def = 今の既定 (環境変数なし)、sf = CMIR の効き目の評価を HiGHS の形にする速い版 (`ENOMOTO_MIP_SEP_FAST=1`、
カットは同じ。analysis/sepspeed_20261023.md、analysis/gate2_20261117/sqface/README.md)。"""
CFGS = {
    "def": {},
    "sf": {"ENOMOTO_MIP_SEP_FAST": "1"},
}
