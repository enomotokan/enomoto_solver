"""対の比べの設定 (LPINF: def/old、対称性: def/symf): def = 既定 (際どいノードの LP の実行不能を信用しない直し、558d7a4ce)、old = 直しなし (ENOMOTO_MIP_NO_LPINF_CHECK=1)。"""
CFGS = {
    "def": {},
    "old": {"ENOMOTO_MIP_NO_LPINF_CHECK": "1"},
    # 対称性の対の比べ (SP・SM、12 節): 既定 (orbital fixing だけ) と以前の既定 SYMDYN=f (orbital fixing + LexRed)
    "symf": {"ENOMOTO_MIP_SYMDYN": "f"},
    # カットの係数の締めの直し (analysis/slowm_20261119、WRONG2) の対の比べ (PT): 既定 (直しあり) と直しなし
    "tib0": {"ENOMOTO_MIP_TIGHTEN_INTBOUND": "0"},
    # 分離器に渡す大域の境界を暗黙の整数列だけ丸める直し (同、b6762abe3) の対の比べ (PR): 既定 (丸める) と丸めない
    "sri0": {"ENOMOTO_MIP_SEP_ROUND_IMPLINT": "0"},
}
