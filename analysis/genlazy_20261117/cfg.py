"""試作 `ENOMOTO_MIP_GEN_LAZY` (余裕のある一般の不等式の行を LP から外し、破られたら戻す) の設定。
tNN = θ (正規化した活動のしきい値) が 0.NN、`sb` は強分岐の子の LP 解でも外した行を確かめる (`ENOMOTO_MIP_GEN_LAZY_SB=1`)、
`clq` は CLQ_LAZY と同じくクリーク型の行だけ (θ = 1 で、余裕のあるクリーク型の行を全部)。lazy の設定は診断の行 (GENLAZY) も出す。"""
def g(theta, kinds="1", sb=False):
    e = {"ENOMOTO_MIP_GEN_LAZY": kinds, "ENOMOTO_T_MIP_GEN_LAZY_THETA": str(theta), "ENOMOTO_MIP_DIAG_GENLAZY": "1"}
    if sb:
        e["ENOMOTO_MIP_GEN_LAZY_SB"] = "1"
    return e
CFGS = {
    "def": {},
    "t03": g(0.3), "t05": g(0.5), "t08": g(0.8), "t10": g(1.0),
    "t05sb": g(0.5, sb=True), "t10sb": g(1.0, sb=True),
    "clq": g(1.0, "clq"),
    "diag": {"ENOMOTO_MIP_GEN_LAZY": "1", "ENOMOTO_MIP_DIAG_GENLAZY": "stop"},
}
