"""gate2 候補 2 (CLQPRB の s) の設定: def = 今の既定 (環境変数なし)、clqs = プロービングの含意を後の伝播に使う形 (`CLQPRB=1dgr`) に、
成果の比のシグモイドで既定の形に戻す `s` を足したもの (analysis/gate2_20261117/clqprb_s/README.md)。"""
CFGS = {
    "def": {},
    "clqs": {"ENOMOTO_MIP_CLQPRB": "1dgrs"},
}
