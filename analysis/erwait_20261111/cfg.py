"""ERWAIT の設定。どの回も診断 ENOMOTO_MIP_ERWAIT_DIAG=1 (ログを出すだけ、振る舞いは変えない) を付ける。"""
D = {"ENOMOTO_MIP_ERWAIT_DIAG": "1"}
CFGS = {
    "def": {**D},                                              # 既定
    "es": {**D, "ENOMOTO_MIP_LOSE6_1_STOP": "w"},              # 既存の試作 (サブ MIP を止め、裏のカットの待ちを 2 秒まで)
    "ew": {**D, "ENOMOTO_MIP_ERWAIT": "sbc"},                  # 試作: サブ MIP・裏のカットのラウンド・カットのループの続きを止める
    "ewk": {**D, "ENOMOTO_MIP_ERWAIT": "sbkc"},                # ew + k: 裏のカットを 2 秒まで待ってから止め、最後に終わったラウンドで置き換える
}
