"""sbdegen の試作 (analysis/sbdegen_20261126/README.md) の P での比べの設定。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。
形は analysis/lpprobe_20261126/wide/cfg.py と同じ (run.py・summ.py・plan.py はその写し)。
- 既定 (def) と `ENOMOTO_MIP_SBDEGEN=1` (sbd、強分枝の予算を効き目のシグモイドで既定と HiGHS・SCIP の大きさの間に置く)。shards.json・res_sK.jsonl。"""
CFGS = {"def_t1": {}, "def_t4": {},
        "sbd_t1": {"ENOMOTO_MIP_SBDEGEN": "1"}, "sbd_t4": {"ENOMOTO_MIP_SBDEGEN": "1"}}
P1 = {1: ("sbd", CFGS["sbd_t1"], True, True, "sbdegen_20261126")}
PLAN = ["def_t1", "def_t4", "sbd_t1", "sbd_t4"]
