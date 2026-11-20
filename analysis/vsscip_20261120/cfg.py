"""SCIP との比べの測り直し (2026-11-20、利用者の依頼)。def: 当方の既定 4 スレッド。def_t1: 当方の既定 1 スレッド (run.py の TH)。
SCIP は回さない (11/17 の analysis/vsscip_20261117 の scip の行を同じ機械・同じ種のものとして使い回す、summ.py)。"""
CFGS = {"def": {}, "def_t1": {}, "scip": {}}
