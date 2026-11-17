"""net12 の Enumeration の試作の対の比べ (treeclose 9.54 節、2026-11-17)。
def = 既定 (列挙なし)、ab = `ENOMOTO_MIP_HO3_ENUM=ab` (全部の候補の行を列挙、前の回で何も見つからなければ次は飛ばす (a)、
最後に見つけてから 0.2 秒見つからなければやめる (b))、ho1 = `ENOMOTO_MIP_HO3_ENUM=1` (元の試作、presolve2 で時間の費用で不採用)。"""
CFGS = {"def": {}, "ab": {"ENOMOTO_MIP_HO3_ENUM": "ab"}, "ho1": {"ENOMOTO_MIP_HO3_ENUM": "1"}}
