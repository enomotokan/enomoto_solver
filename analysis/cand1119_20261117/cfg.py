"""束 cand1119 (2026-11-17、案)。既定 (= cand1118、analysis/cand1118_default_20261116) に次を足したもの (環境変数だけ、既定は変えない):
- クリークのカットのシグモイド (analysis/rootgap_20261116/clq の 2〜5 節): ENOMOTO_MIP_CLIQUE_CUTS=sigmoid・ENOMOTO_T_MIP_CLIQUE_SIG_A=1.8・ENOMOTO_T_MIP_CLIQUE_SIG_B=0.41
- 双子の併合 (同じ README の 6 節): ENOMOTO_MIP_SYM_TWINS=1
- カットの選び方の下見の手間の見込み (analysis/cutprobe_20261117 の aff): ENOMOTO_MIP_CUTSEL_PROBE_AFFORD=1
def = 既定 (環境変数なし)、cand1119 = 上を足したもの。"""
CAND1119 = {"ENOMOTO_MIP_CLIQUE_CUTS": "sigmoid", "ENOMOTO_T_MIP_CLIQUE_SIG_A": "1.8", "ENOMOTO_T_MIP_CLIQUE_SIG_B": "0.41",
            "ENOMOTO_MIP_SYM_TWINS": "1", "ENOMOTO_MIP_CUTSEL_PROBE_AFFORD": "1"}
CFGS = {"def": {}, "cand1119": CAND1119}
