"""gate2 候補 1 の設定: def = 今の既定 (環境変数なし)、tsig = 試作 ENOMOTO_MIP_TAB_HIGHS_SIG=1 (analysis/gate2_20261117/tab/README.md)。"""
CFGS = {
    "def": {},
    # 根の最初の tableau の分離の作業量 W のシグモイド (中心 2.5) で HiGHS の行の選び方に切り替える。fill の予算を超える行が 9 割以上なら既定のまま
    "tsig": {"ENOMOTO_MIP_TAB_HIGHS_SIG": "1"},
    # 確かめの計測 (README 12 節) の札。中身は tsig と同じ環境変数だが、連続列の割合 < 0.6 を足した版 (2f6a971c9 以降) で回す
    "tsig2": {"ENOMOTO_MIP_TAB_HIGHS_SIG": "1"},
}
