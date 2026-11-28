"""T1PRIMAL (1 スレッドの解探しの手間の配分、src/mip/t1primal.rs) の比べの設定。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。
形は analysis/unify_20261126/cfg.py と同じ (summ.py・plan.py・run.py はその写し)。
- tp: `ENOMOTO_MIP_T1PRIMAL=1` (既定オフの試作)。
- P (plan.py → shards_P.json): MIPLIB の 58 問 × 種 1〜3 × {def_t1, tp_t1}、60 秒。
- L (shards_L.json、手で書いた): 狙いの 4 問 × 種 1〜3 × {def_t1, tp_t1}、600 秒。集計は `T1P_TL=600 python summ.py 'res_L_s*.jsonl'`。
- 5 版目を新しい既定 (d5956146f 以降) で測り直す (README 10 節): P は shards_P5n.json (3 分担、(問題, 種) ごとに def と tp を同じ分担)、
  L は shards_L5n.json (2 分担) で {def, tp, heurfp, heurfp + tp} の 4 つ。heurfp は analysis/base1_20261128/cfg.py と同じ
  (HEUR_ADAPT=1mkr + FP_STAGE3_LS=a)。"""
CFGS = {"def_t1": {}, "def_t4": {},
        "tp_t1": {"ENOMOTO_MIP_T1PRIMAL": "1"}, "tp_t4": {"ENOMOTO_MIP_T1PRIMAL": "1"},
        "heurfp_t1": {"ENOMOTO_MIP_HEUR_ADAPT": "1mkr", "ENOMOTO_MIP_FP_STAGE3_LS": "a"},
        "heurfptp_t1": {"ENOMOTO_MIP_HEUR_ADAPT": "1mkr", "ENOMOTO_MIP_FP_STAGE3_LS": "a", "ENOMOTO_MIP_T1PRIMAL": "1"}}
P1 = {1: ("tp", CFGS["tp_t1"], True, True, "t1primal_20261126"),
      2: ("heurfp", CFGS["heurfp_t1"], True, False, "t1primal_20261126"),
      3: ("heurfptp", CFGS["heurfptp_t1"], True, False, "t1primal_20261126")}
PLAN = ["def_t1", "tp_t1"]
