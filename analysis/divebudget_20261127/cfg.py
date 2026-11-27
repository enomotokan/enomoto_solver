"""divebudget の設定 (analysis/divebudget_20261127)。def: 今の既定。db: 試作 ENOMOTO_MIP_DIVEBUDGET=1 (木の中のダイビングの LP 反復の上限を種類ごと、
SCIP の getDivesetIterLimit・MINLPITER・lpsolvefreq 0)。名前が _t1 で終わるものは 1 スレッド (run.py の TH)、ほかは 4 スレッド。
DIVEBUDGET_STAT は統計の行を出すだけ (解き方は変えない)。"""
ST = {"ENOMOTO_MIP_DIVEBUDGET_STAT": "1"}
DB = {"ENOMOTO_MIP_DIVEBUDGET": "1"}
CFGS = {"def": ST, "db": DB, "def_t1": ST, "db_t1": DB}
