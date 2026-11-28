"""ROOTRESTART2 (analysis/rootrestart_20261126/README.md の 4 節) の P での比べの設定: 既定 (def) と試作を 1・4 スレッドで。
名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。形は analysis/polishp_20261125/cfg.py と同じ。
- rr2: `ENOMOTO_MIP_ROOTRESTART2=eam` (7・8 節、shards.json)
- am: `ENOMOTO_MIP_ROOTRESTART2=am` (9 節、部品 e を外した形、shards3.json)
- rl: `ENOMOTO_MIP_RSTLOOSE=tpci` (10 節、再スタートの閾値を緩め引き継ぎを足す形、shards4.json)
集計 (summ.py) で見る試作は環境変数 RR_P で選ぶ (既定は rr2。9 節は RR_P=am、10 節は RR_P=rl)。"""
import os
_P = {"rr2": {"ENOMOTO_MIP_ROOTRESTART2": "eam"}, "am": {"ENOMOTO_MIP_ROOTRESTART2": "am"},
      "rl": {"ENOMOTO_MIP_RSTLOOSE": "tpci"}}
_name = os.environ.get("RR_P", "rr2")
P1 = {1: (_name, _P[_name], True, True, "rootrestart_20261126")}
CFGS = {"def_t1": {}, "def_t4": {}}
for _n, _v in _P.items():
    CFGS[f"{_n}_t1"] = CFGS[f"{_n}_t4"] = _v
