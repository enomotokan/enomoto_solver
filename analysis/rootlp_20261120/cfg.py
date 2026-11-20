"""ROOTLP の設定 (analysis/rootlp_20261120/README.md)。名前が _t1 で終われば 1 スレッド、ほかは 4 スレッド (run.py の TH)。
- def / def_t1: 既定。
- ncnr / ncnr_t1: 目的が全て 0 の問題の早めの FJ/Local-MIP で連鎖の手を使わない (ENOMOTO_MIP_FEAS_EARLY_LMIP_NO_CHAIN) +
  早めの探索が解なしで上限まで回ったら根の後のやり直し FJ を省く (ENOMOTO_MIP_FEAS_NORETRY)。
- rhb: 根の Pump・ダイビングの LP 反復の予算を、それまでの LP の反復の割合 (SCIP の形) にする (ENOMOTO_MIP_RHBUDGET、README 4 節)。"""
NCNR = {"ENOMOTO_MIP_FEAS_EARLY_LMIP_NO_CHAIN": "1", "ENOMOTO_MIP_FEAS_NORETRY": "1"}
CFGS = {
    "def": {},
    "def_t1": {},
    "ncnr": NCNR,
    "ncnr_t1": NCNR,
    "rhb": {"ENOMOTO_MIP_RHBUDGET": "1"},
}
