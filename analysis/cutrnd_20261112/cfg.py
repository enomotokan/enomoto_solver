"""CUTRND の設定 (試作 ENOMOTO_MIP_CUTRND、analysis/cutrnd_20261112.md)。"""
CFGS = {
    "def": {},                                   # 既定
    "e": {"ENOMOTO_MIP_CUTRND": "e"},            # ラウンドの上限に当たったら、伸びと時間の比で延ばす
    "es": {"ENOMOTO_MIP_CUTRND": "es"},          # e + 長く回って伸びが細ったら止める
    "ef": {"ENOMOTO_MIP_CUTRND": "ef"},          # e + 動かないラウンドを停滞 1 回と数える
    "esf": {"ENOMOTO_MIP_CUTRND": "esf"},
    "cr": {"ENOMOTO_MIP_CUTRND": "esf"},          # 58 問の害の確かめに使う候補 (esf と同じ)
    "c100": {"ENOMOTO_T_MIP_CUT_ROUNDS": "100"},  # 比べ: ラウンドの上限を 100 にするだけ
}
