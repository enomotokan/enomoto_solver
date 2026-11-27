"""LSDIVE (SCIP の linesearchdiving に当たる木の中のダイビング、src/mip/heuristics.rs の `DiveKind::LineSearch`) の比べの設定。
名前が _t1 で終われば 1 スレッド、そうでなければ 4 スレッド (run.py の TH)。形は analysis/t1primal_20261126/cfg.py と同じ。
- ls: `ENOMOTO_MIP_LSDIVE=1` (既定オフの試作)。tp: `ENOMOTO_MIP_T1PRIMAL=1` (HsTC の試作)。lstp: 両方。
- L (shards_L.json): 狙いの 4 問 × 種 1〜3 × {def_t1, ls_t1, tp_t1, lstp_t1}、600 秒。
- P (plan.py → shards_P.json): MIPLIB の 58 問 × 種 1〜3 × PLAN、60 秒。"""
CFGS = {"def_t1": {}, "def_t4": {},
        "ls_t1": {"ENOMOTO_MIP_LSDIVE": "1"}, "ls_t4": {"ENOMOTO_MIP_LSDIVE": "1"},
        "tp_t1": {"ENOMOTO_MIP_T1PRIMAL": "1"},
        "lstp_t1": {"ENOMOTO_MIP_LSDIVE": "1", "ENOMOTO_MIP_T1PRIMAL": "1"}}
P1 = {1: ("ls", CFGS["ls_t1"], True, True, "lsdive_20261127")}
PLAN = ["def_t1", "ls_t1", "def_t4", "ls_t4"]
