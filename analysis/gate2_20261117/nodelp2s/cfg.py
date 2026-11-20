"""gate2 候補 4 (NODELP2 の作り直し) の設定: def = 今の既定 (環境変数なし)、xs = 試作 `ENOMOTO_MIP_NODELP2=xs`
(取り出したノードの最初の LP の DSE 重みを厳密/保存から時間の移動平均で選ぶ `x` を、ノードの LP の平均の反復のシグモイドの割合で使う。README.md)。"""
CFGS = {
    "def": {},
    "xs": {"ENOMOTO_MIP_NODELP2": "xs"},
}
