"""ROOTRESTART2 (analysis/rootrestart_20261126/README.md の 4 節) の P での比べの設定: 既定 (def) と `ENOMOTO_MIP_ROOTRESTART2=eam` (rr2) を
1・4 スレッドで。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。形は analysis/polishp_20261125/cfg.py と同じ。"""
P1 = {1: ("rr2", {"ENOMOTO_MIP_ROOTRESTART2": "eam"}, True, True, "rootrestart_20261126")}
CFGS = {"def_t1": {}, "def_t4": {}, "rr2_t1": P1[1][1], "rr2_t4": P1[1][1]}
