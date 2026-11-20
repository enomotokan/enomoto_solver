"""OBJTARGET の計測の設定 (README)。def = 今の既定。l = Local-MIP の目的の行の上限を「最良 − max(刻み, α (最良 − L))」に置く試作 (ENOMOTO_MIP_OBJTARGET=l)。
p = 前処理と並行して PDLP (2000 反復) の点から FJ を回し、解を開始解にする試作 (ENOMOTO_MIP_TPRE=p)。"""
CFGS = {"def": {}, "l": {"ENOMOTO_MIP_OBJTARGET": "l"}, "p": {"ENOMOTO_MIP_TPRE": "p"}}
