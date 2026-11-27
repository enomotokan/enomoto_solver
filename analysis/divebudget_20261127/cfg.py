"""divebudget の設定 (analysis/divebudget_20261127)。def: 今の既定。db: 試作 ENOMOTO_MIP_DIVEBUDGET=1 (木の中のダイビングの LP 反復の上限を種類ごと、
SCIP の getDivesetIterLimit・MINLPITER・lpsolvefreq 0)。u・ub: 試作 ENOMOTO_MIP_DIVESEL (種類を UCB1 で選ぶ)。名前が _t1 で終わるものは 1 スレッド (run.py の TH)、ほかは 4 スレッド。
DIVEBUDGET_STAT は統計の行を出すだけ (解き方は変えない)。"""
ST = {"ENOMOTO_MIP_DIVEBUDGET_STAT": "1"}
DB = {"ENOMOTO_MIP_DIVEBUDGET": "1"}
# u: 種類を UCB1 で選ぶだけ (上限は今の共有のまま)。ub: UCB1 と種類ごとの上限 (DIVEBUDGET と同じ)
U = {"ENOMOTO_MIP_DIVESEL": "u"}
UB = {"ENOMOTO_MIP_DIVESEL": "ub"}
# ubg: ub に、上限の割合 0.05 へギャップの係数 σ(logit(laplace(ギャップ, n))) を掛ける (T1PRIMAL v2 の g と同じ式、src/mip/adapt.rs)
UBG = {"ENOMOTO_MIP_DIVESEL": "ubg"}
CFGS = {"def": ST, "db": DB, "def_t1": ST, "db_t1": DB, "u": U, "u_t1": U, "ub": UB, "ub_t1": UB, "ubg": UBG, "ubg_t1": UBG}
