"""SCIP-SYM の設定 (analysis/scipsym_20261120/README.md)。名前が _t1 で終われば 1 スレッド、ほかは 4 スレッド (run.py の TH)。
- def / def_t1: このコミットの既定。
- xo / xo_t1: 試作 ENOMOTO_XO_BUMP_LU_RATIO=1 (README 13 節)。
- pplex / pplex_t1: 試作 ENOMOTO_MIP_SYM_PPLEX=1 (詰め込みの行に入る対合の多い成分を、対合ごとの 2 列の静的な詰め込みオービトープで扱う)。"""
CFGS = {
    "def": {},
    "def_t1": {},
    "pplex": {"ENOMOTO_MIP_SYM_PPLEX": "1"},
    "pplex_t1": {"ENOMOTO_MIP_SYM_PPLEX": "1"},
    # xo / xo_t1: 試作 ENOMOTO_XO_BUMP_LU_RATIO=1 (クロスオーバーの Megiddo の押し出しの fill の上限を双対単体法と同じく FT_BUMP_LU_RATIO·nnz(LU) に、README 13 節)
    "xo": {"ENOMOTO_XO_BUMP_LU_RATIO": "1"},
    "xo_t1": {"ENOMOTO_XO_BUMP_LU_RATIO": "1"},
}
