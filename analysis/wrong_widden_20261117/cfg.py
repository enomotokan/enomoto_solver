"""対の比べの設定 (LPINF: def/old、対称性: def/symf): def = 既定 (際どいノードの LP の実行不能を信用しない直し、558d7a4ce)、old = 直しなし (ENOMOTO_MIP_NO_LPINF_CHECK=1)。"""
CFGS = {
    "def": {},
    "old": {"ENOMOTO_MIP_NO_LPINF_CHECK": "1"},
    # 対称性の対の比べ (SP・SM、12 節): 既定 (orbital fixing だけ) と以前の既定 SYMDYN=f (orbital fixing + LexRed)
    "symf": {"ENOMOTO_MIP_SYMDYN": "f"},
    # カットの係数の締めの直し (analysis/slowm_20261119、WRONG2) の対の比べ (PT): 既定 (直しあり) と直しなし
    "tib0": {"ENOMOTO_MIP_TIGHTEN_INTBOUND": "0"},
}
