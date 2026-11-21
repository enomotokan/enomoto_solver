"""rootstop の設定 (analysis/rootstop_20261121)。
- def: 今の既定。
- long: 根のカットのループを続けた回 (診断)。第 1 段階が上限 (25 巡) に当たったら必ず第 2 段階へ (`ENOMOTO_MIP_CUT_PHASE2_AT_CAP`)、
  第 2 段階の巡の上限 60 → 75 (合わせて 100 巡)・時間の枠 (第 1 段階の時間の 3 倍 → 30 倍)、根のカットの時間の上限 (時間の上限の 0.1 → 0.3)。
  停滞の判定 (下界が動かない) はそのまま。
- rs: 試作 `ENOMOTO_MIP_ROOTSTOP` (直近 5 巡の伸びの割合 < 同じ 5 巡で LP の行が増えた割合 なら止める)。
- short: 逆向きの診断。第 1 段階の巡の上限 25 → 15 (`ENOMOTO_T_MIP_CUT_ROUNDS`)。ほかは既定のまま。
"""
LONG = {"ENOMOTO_MIP_CUT_PHASE2_AT_CAP": "1", "ENOMOTO_T_MIP_CUT_PHASE2_ROUNDS": "75", "ENOMOTO_T_MIP_CUT_PHASE2_TIME_MULT": "30",
        "ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.3"}
CFGS = {"def": {}, "long": LONG, "short": {"ENOMOTO_T_MIP_CUT_ROUNDS": "15"}, "rs": {"ENOMOTO_MIP_ROOTSTOP": "1"}}
