"""featconst 3 (cutlen) の設定: def = 今の既定 (環境変数なし)、cl = 外側の根のカットのループの上限を r (1 ラウンドの手間 ÷ 根の LP の手間) の
シグモイドで 8〜25 ラウンドにする試作 (`ENOMOTO_MIP_CUTLEN=1`、analysis/featconst_20261118/cutlen/README.md)。"""
CFGS = {
    "def": {},
    "cl": {"ENOMOTO_MIP_CUTLEN": "1"},
}
