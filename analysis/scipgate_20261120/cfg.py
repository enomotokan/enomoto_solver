"""SCIP-GATE の計測の設定 (analysis/scipgate_20261120)。どれも 4 スレッド。
def: 今の既定。vb: SCIP-VB の試作 ENOMOTO_MIP_VB01=b (根の LP の前の vbounds。SCIP-VB の計測の vb01 は =1 = 根のヒューリスティクスの後で別物)。
vbc: vb + ENOMOTO_MIP_VBCLQ=1 (根の LP の前の pseudoobj)。vbcl: vb + ENOMOTO_MIP_VBCLQ=l (pseudoobj + 前処理の LP による固定をしない)。
vbce: ENOMOTO_MIP_VBCLQ=e だけ (2 回目の計測、def との対)。"""
CFGS = {
    "def": {},
    "vb": {"ENOMOTO_MIP_VB01": "b"},
    "vbc": {"ENOMOTO_MIP_VB01": "b", "ENOMOTO_MIP_VBCLQ": "1"},
    "vbcl": {"ENOMOTO_MIP_VB01": "b", "ENOMOTO_MIP_VBCLQ": "l"},
    # 本命 (README 8 節): 前処理の LP による固定の前に VB01 の解と pseudoobj で固定し、前処理をかけ直す (根の前の VB01=b は付けない)
    "vbce": {"ENOMOTO_MIP_VBCLQ": "e"},
}
