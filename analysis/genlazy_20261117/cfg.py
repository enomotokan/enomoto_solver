"""試作 `ENOMOTO_MIP_GEN_LAZY` (余裕のある一般の不等式の行を LP から外し、破られたら戻す) の設定。
tNN = θ (正規化した活動のしきい値) が 0.NN、`sb` は強分岐の子の LP 解でも外した行を確かめる (`ENOMOTO_MIP_GEN_LAZY_SB=1`)、
`clq` は CLQ_LAZY と同じくクリーク型の行だけ (θ = 1 で、余裕のあるクリーク型の行を全部)。lazy の設定は診断の行 (GENLAZY) も出す。
sNN は点数の判定 (`ENOMOTO_MIP_GEN_LAZY_MODE=score`、カットの質と同じ点数 < τ で外す。s00 = τ 0、sm01 = τ −0.1、sm03 = τ −0.3)、
`c` 付きはクリーク型だけ、`aNN` 付きは戻した行を年齢 NN で外し直す (`ENOMOTO_T_MIP_GEN_LAZY_AGE`)。"""
def g(theta, kinds="1", sb=False, tau=None, age=0):
    e = {"ENOMOTO_MIP_GEN_LAZY": kinds, "ENOMOTO_T_MIP_GEN_LAZY_THETA": str(theta), "ENOMOTO_MIP_DIAG_GENLAZY": "1"}
    if sb:
        e["ENOMOTO_MIP_GEN_LAZY_SB"] = "1"
    if tau is not None:
        e["ENOMOTO_MIP_GEN_LAZY_MODE"] = "score"; e["ENOMOTO_T_MIP_GEN_LAZY_TAU"] = str(tau)
    if age:
        e["ENOMOTO_T_MIP_GEN_LAZY_AGE"] = str(age)
    return e
CFGS = {
    "def": {},
    "t01": g(0.1), "t03": g(0.3), "t05": g(0.5), "t08": g(0.8), "t10": g(1.0),
    "t05sb": g(0.5, sb=True), "t10sb": g(1.0, sb=True),
    "clq": g(1.0, "clq"),
    "s00": g(0, tau=0.0), "sm01": g(0, tau=-0.1), "sm03": g(0, tau=-0.3),
    "sm1": g(0, tau=-1.0), "sm3": g(0, tau=-3.0), "sm100": g(0, tau=-100.0), "sm1000": g(0, tau=-1000.0),
    "s00c": g(0, "clq", tau=0.0), "sm01c": g(0, "clq", tau=-0.1),
    "s00a50": g(0, tau=0.0, age=50), "t05a50": g(0.5, age=50),
    "sm1touch": {**g(0, tau=-1.0), "ENOMOTO_MIP_GEN_LAZY_TOUCH": "1"}, "sm03touch": {**g(0, tau=-0.3), "ENOMOTO_MIP_GEN_LAZY_TOUCH": "1"},
    # mzzv11 の切り分け: 1 行外す形 (sm1) で、空の行の扱いを 1 つずつ元に戻す (`ENOMOTO_MIP_FREEROW_OFF`)
    **{f"sm1off{c}": {**g(0, tau=-1.0), "ENOMOTO_MIP_FREEROW_OFF": c} for c in "rdsp"},
    "fixrow": {"ENOMOTO_MIP_FIXROW": "1"},
    "diag": {"ENOMOTO_MIP_GEN_LAZY": "1", "ENOMOTO_MIP_DIAG_GENLAZY": "stop"},
}
