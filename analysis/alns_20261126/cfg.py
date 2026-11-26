"""alns の設定 (analysis/alns_20261126)。def: 今の既定。alns: 試作 ENOMOTO_MIP_ALNS=1 (木の大近傍探索を CP-SAT の 3 本の近傍に替える)。
名前が _t1 で終わるものは 1 スレッド (run.py の TH)、ほかは 4 スレッド。"""
ALNS = {"ENOMOTO_MIP_ALNS": "1"}
CFGS = {"def": {}, "alns": ALNS, "def_t1": {}, "alns_t1": ALNS}
