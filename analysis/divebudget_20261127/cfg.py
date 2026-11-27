"""divebudget の設定 (analysis/divebudget_20261127)。def: 今の既定。db: 試作 ENOMOTO_MIP_DIVEBUDGET=1 (木の中のダイビングの LP 反復の上限を種類ごと、
SCIP の getDivesetIterLimit・MINLPITER・lpsolvefreq 0)。u・ub: 試作 ENOMOTO_MIP_DIVESEL (種類を UCB1 で選ぶ)。名前が _t1 で終わるものは 1 スレッド (run.py の TH)、ほかは 4 スレッド。
DIVEBUDGET_STAT は統計の行を出すだけ (解き方は変えない)。
LU_REFAC_BAL=0: 751b2be62 で LU_REFAC_BAL=k が既定になった。P の 4 スレッドの def・db・ub (FMjE、671a25f91) はその前の版で回したので、
後の版で回す分も前の道筋にそろえる (前の基準と一致することを check_default で確かめる)。"""
ST = {"ENOMOTO_MIP_DIVEBUDGET_STAT": "1"}
DB = {"ENOMOTO_MIP_DIVEBUDGET": "1"}
# u: 種類を UCB1 で選ぶだけ (上限は今の共有のまま)。ub: UCB1 と種類ごとの上限 (DIVEBUDGET と同じ)
U = {"ENOMOTO_MIP_DIVESEL": "u"}
UB = {"ENOMOTO_MIP_DIVESEL": "ub"}
# ubg: ub に、上限の割合 0.05 へギャップの係数 2·laplace(ギャップ, その種類の回数) を掛ける (T1PRIMAL v2 の g と同じならし方、src/mip/adapt.rs)
UBG = {"ENOMOTO_MIP_DIVESEL": "ubg"}
L0 = {"ENOMOTO_LU_REFAC_BAL": "0"}
_C = {"def": ST, "db": DB, "def_t1": ST, "db_t1": DB, "u": U, "u_t1": U, "ub": UB, "ub_t1": UB, "ubg": UBG, "ubg_t1": UBG}
CFGS = {k: {**L0, **v} for k, v in _C.items()}
