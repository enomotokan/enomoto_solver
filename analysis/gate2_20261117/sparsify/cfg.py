"""gate2 候補 6 (SPARSIFY_LATE の作り直し) の設定: def = 今の既定 (環境変数なし)、spls = 試作 `ENOMOTO_MIP_SPARSIFY_LATE=s`
(前処理の最後の疎化で書き換えた行のうち 1 列以下に縮んだ割合 f のシグモイドで、疎化した問題を使うかを決める。README.md)。"""
CFGS = {
    "def": {},
    "spls": {"ENOMOTO_MIP_SPARSIFY_LATE": "s"},
}
