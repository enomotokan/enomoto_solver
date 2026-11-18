"""cutcoef 段 1 の設定: カットの点数 = 効き目 + wo × 目的との平行度 + wi × 整数の割合、を 非零^α で割る (src/mip/separation.rs の cut_quality・cut_score)。
名前は o<wo>i<wi>a<α> (0.1 は 01、0.5 は 05)。def = 今の既定 (wo = wi = 0.1、α = 0、環境変数なし)。
格子: (wo, wi) ∈ {0, 0.1, 1}² を α = 0 で (9 組、うち 1 組は def)、α ∈ {0.5, 1} を (0.1, 0.1) で (2 組)。計 11 組。"""
def tag(x): return {0.0: "0", 0.1: "01", 0.5: "05", 1.0: "1"}[x]
CFGS = {}
for wo in (0.0, 0.1, 1.0):
    for wi in (0.0, 0.1, 1.0):
        n = "def" if (wo, wi) == (0.1, 0.1) else f"o{tag(wo)}i{tag(wi)}a0"
        CFGS[n] = {} if n == "def" else {"ENOMOTO_T_CUTSEL_W_OBJPAR": str(wo), "ENOMOTO_T_CUTSEL_W_INTSUP": str(wi)}
for a in (0.5, 1.0):
    CFGS[f"o01i01a{tag(a)}"] = {"ENOMOTO_T_CUTSEL_DENSITY_EXP": str(a)}
