"""DUALEQ2 (2026-11-27) の設定。def: 今の既定。deq: 試作 ENOMOTO_MIP_DUALEQ=1 (前処理の最後に双対の議論で不等式行を等式にし、
かけ直しで縮んだときだけ使う。analysis/presolve2_20261111)。名前が _t1 で終わるものは 1 スレッド (run.py の TH)、ほかは 4 スレッド。"""
CFGS = {"def": {}, "deq": {"ENOMOTO_MIP_DUALEQ": "1"}, "def_t1": {}, "deq_t1": {"ENOMOTO_MIP_DUALEQ": "1"}}
