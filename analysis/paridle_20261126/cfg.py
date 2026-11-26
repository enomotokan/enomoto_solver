"""paridle の試作の P の計測 (README 3 節) の設定。名前が _t4 なら 4 スレッド (run.py の TH)。
試作は `ENOMOTO_MIP_PARIDLE` の文字 (src/mip/solver.rs `paridle`)。`h` = 準備を終えて待っている従がいれば主の控えを終える、`e` = 木の最初のノードの前に従を始める。"""
P1 = {1: ("h", {"ENOMOTO_MIP_PARIDLE": "h"}, False, True, "paridle_20261126"),
      2: ("eh", {"ENOMOTO_MIP_PARIDLE": "eh"}, False, True, "paridle_20261126")}
CFGS = {"def_t4": {}}
for k, (name, env, t1, t4, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
