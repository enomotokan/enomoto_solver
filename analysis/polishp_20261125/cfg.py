"""POLISH_STRICT (analysis/nattai_20261125) の P での比べ (2026-11-25、親の依頼) の設定。既定 (def_t1・def_t4) と STRICT=1・=2 を
1・4 スレッドで。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。形は analysis/protoscreen_20261124/cfg.py と同じ (summ.py をそのまま使う)。"""
P1 = {  # 番号: (名前, 環境変数, 1 スレッドで回すか, 4 スレッドで回すか, 記録)
    1: ("s1", {"ENOMOTO_MIP_POLISH_STRICT": "1"}, True, True, "nattai_20261125"),
    2: ("s2", {"ENOMOTO_MIP_POLISH_STRICT": "2"}, True, True, "nattai_20261125"),
}
CFGS = {"def_t1": {}, "def_t4": {}}
for k, (name, env, t1, t4, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
# 超過を抑える試作 `ENOMOTO_MIP_POLISH_WARM=d` の小さな確かめ (2026-11-25、shards_small.json・res_small.jsonl、summ.py の対象外)
CFGS["s1wd_t1"] = {"ENOMOTO_MIP_POLISH_STRICT": "1", "ENOMOTO_MIP_POLISH_WARM": "d"}
CFGS["s2wd_t1"] = {"ENOMOTO_MIP_POLISH_STRICT": "2", "ENOMOTO_MIP_POLISH_WARM": "d"}
# P 全体での WARM=d の比べ (2026-11-25、親の依頼、既定にする前の最後の確かめ): shards_wd.json・res_wd_sK.jsonl。
# 集計は `POLISHP_SET=wd python summ.py 'res_wd_s*.jsonl'` (P1 を s1wd・s2wd に差し替える)
CFGS["s1wd_t4"] = CFGS["s1wd_t1"]
CFGS["s2wd_t4"] = CFGS["s2wd_t1"]
import os as _os
if _os.environ.get("POLISHP_SET") == "wd":
    P1 = {
        1: ("s1wd", CFGS["s1wd_t1"], True, True, "nattai_20261125"),
        2: ("s2wd", CFGS["s2wd_t1"], True, True, "nattai_20261125"),
    }
    CFGS = {k: CFGS[k] for k in ("def_t1", "def_t4", "s1wd_t1", "s1wd_t4", "s2wd_t1", "s2wd_t4")}
