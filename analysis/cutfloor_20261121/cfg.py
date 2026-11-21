"""cutfloor の広い計測の設定 (analysis/cutfloor_20261121)。finish_cut の直し (analysis/rootgap_20261121/wrong_widden) の既定の道での影響を見る。
def: 直しなし (小さい係数の吸収の下限 ENOMOTO_MIP_CUT_SMALL_FLOOR=0 と、打ち消しの確かめ ENOMOTO_MIP_FINISH_CANCEL=0 を切る = 以前の動き)。
fix: 直しあり (環境変数なし)。どれも 4 スレッド (run.py の TH)。"""
CFGS = {"def": {"ENOMOTO_MIP_CUT_SMALL_FLOOR": "0", "ENOMOTO_MIP_FINISH_CANCEL": "0"}, "fix": {}}
