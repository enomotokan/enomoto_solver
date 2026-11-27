"""lpprobe の直し (analysis/lpprobe_20261126/README.md) の P での比べの設定。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。
形は analysis/unify_20261126/cfg.py と同じ (run.py・summ.py・plan.py はその写し)。
- 第 1 弾: 既定 (def) と `ENOMOTO_MIP_LPPROBE_FIX=d` (d、子の LP の厳密な DSE 重みの使い回し、結果を変えない直し)。shards.json・res_sK.jsonl。"""
CFGS = {"def_t1": {}, "def_t4": {},
        "d_t1": {"ENOMOTO_MIP_LPPROBE_FIX": "d"}, "d_t4": {"ENOMOTO_MIP_LPPROBE_FIX": "d"}}
P1 = {1: ("d", CFGS["d_t1"], True, True, "lpprobe_20261126")}
PLAN = ["def_t1", "def_t4", "d_t1", "d_t4"]
