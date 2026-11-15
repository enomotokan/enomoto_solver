"""OBJINT (2026-11-12) の設定。def = 既定 (環境変数なし)。highs = HiGHS 1.15.1 (種は random_seed)。"""
CFGS = {
    "def": {},
    "oi": {"ENOMOTO_MIP_OBJINT": "1"},                                  # 目的値の刻みに HO2 の暗黙の整数と整数値に固定された列を使う
    "oig": {"ENOMOTO_MIP_OBJINT": "1", "ENOMOTO_MIP_HO2_GOBJ": "1"},    # + 打ち切り値による目的関数の行の大域的な伝播 (既存の試作)
}
# 診断の回でだけ足すログ (解き方は変えない)
LOGENV = {"ENOMOTO_MIP_LOG": "1"}
# この機械で回す追加の組 (X): TREEEFF の tenc (= TREEEFF=rpsh + NODECNT=hr、cand1113 の候補) との組み合わせ
CFGS["tenc"] = {"ENOMOTO_MIP_TREEEFF": "rpsh", "ENOMOTO_MIP_NODECNT": "hr"}
CFGS["oitenc"] = {**CFGS["oi"], **CFGS["tenc"]}
# `c` (連続列が混ざっても有界なら目的値の範囲を使う・固定の列は定数へ・根の大域的な固定の後に計算し直す) を足した形
CFGS["oic"] = {"ENOMOTO_MIP_OBJINT": "hfc"}
