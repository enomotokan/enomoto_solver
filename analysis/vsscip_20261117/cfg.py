"""0.1.9 の既定と SCIP の比べ (2026-11-17、利用者の依頼「SCIP との比較も頼む」)。
def: 当方の既定 4 スレッド。def_t1: 当方の既定 1 スレッド (ENOMOTO_MIP_THREADS=1、run.py の TH)。scip: pyscipopt の wheel 同梱の SCIP (既定で逐次)。"""
CFGS = {"def": {}, "def_t1": {}, "scip": {}}
