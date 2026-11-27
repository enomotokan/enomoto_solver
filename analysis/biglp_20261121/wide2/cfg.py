"""wide2 (ROOTCAP=k の直し 632b587b9 の後の P の回し直し、wide/ の写し): biglp の直す案の広い計測の設定 (analysis/biglp_20261121 7 節)。def: 今の既定。fix: 案 5・6・7 の試作を全部
(ENOMOTO_MIP_CUTSEL_PROBE_FIT=1 + ENOMOTO_MIP_ROOTLP3=e + ENOMOTO_MIP_ROOTCAP=k)。fit・r3e・rck は 1 つずつ (分けて回すとき用、
plan.py の CF を書き換える)。どれも 4 スレッド (run.py の TH)。"""
FIT = {"ENOMOTO_MIP_CUTSEL_PROBE_FIT": "1"}
R3E = {"ENOMOTO_MIP_ROOTLP3": "e"}
RCK = {"ENOMOTO_MIP_ROOTCAP": "k"}
CFGS = {"def": {}, "fix": {**FIT, **R3E, **RCK}, "fit": FIT, "r3e": R3E, "rck": RCK}
CFGS["fr"] = {**FIT, **RCK}  # t79: fix から r3e を除いたもの
