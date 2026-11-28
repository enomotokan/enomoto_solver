"""PRESIZE240 型 A (analysis/presize240_20261127 の 7 節) の P の比べの設定。名前が _t1 で終われば 1 スレッド、そうでなければ 4 スレッド (run.py の TH)。
- zr: `ENOMOTO_MIP_ENUM_ADAPT=z` (列挙を縮みの効率で続ける) と `ENOMOTO_MIP_ROWCLEAN=1` (境界で満たされる行を落とす)。どちらも既定オフの試作。
- zs (8 節、ENUM_ADAPT=1st の既定化 f281fa7b8 の後): `ENOMOTO_MIP_ENUM_ADAPT=1stz` (既定の 1st に z を重ねる) と `ENOMOTO_MIP_ROWCLEAN=1`。PLAN3 (shards_P3.json) で既定と比べる。
- P (plan.py → shards_P.json): MIPLIB の 58 問 × 種 1〜3 × PLAN、60 秒。"""
ZR = {"ENOMOTO_MIP_ENUM_ADAPT": "z", "ENOMOTO_MIP_ROWCLEAN": "1"}
CFGS = {"def_t1": {}, "def_t4": {}, "zr_t1": dict(ZR), "zr_t4": dict(ZR)}
P1 = {1: ("zr", CFGS["zr_t1"], True, True, "presize240_20261127/p")}
PLAN = ["def_t1", "zr_t1", "def_t4", "zr_t4"]
ZS = {"ENOMOTO_MIP_ENUM_ADAPT": "1stz", "ENOMOTO_MIP_ROWCLEAN": "1"}
CFGS.update({"zs_t1": dict(ZS), "zs_t4": dict(ZS)})
PLAN3 = ["def_t1", "zs_t1", "def_t4", "zs_t4"]
