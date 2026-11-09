# GAP2: 根で解ける問題の固定費 (HiGHS が根で数秒で解く問題で ENOMOTO が 2〜7 倍かかる元) (2026-11-09)

bench に使う環境変数: 既定 (`def`) はなし。試作 (どれも既定では使わない) は
`g2` = `ENOMOTO_MIP_GAP2_FLAT=3 ENOMOTO_MIP_GAP2_ZEROCAP=2 ENOMOTO_MIP_GAP2_PROBE1=1 ENOMOTO_MIP_GAP2_LPPGIVE=0.5` (この記録で足した 4 つ)、
`g2x` = `g2` + 既存の `ENOMOTO_MIP_LOSE6_1_STOP=w ENOMOTO_T_MIP_LOSE6_1_BGWAIT=0` (`erbg`) と `ENOMOTO_MIP_PREPAR=ple` (`ple`)。
1 回の計測はどれも `ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=種`・`--threads 4`・上限 600 秒 (58 問は 60 秒)。
HiGHS は highspy 1.15.1 (`--worker-highs`、threads=4・mip_rel_gap=0・output_flag 真・`random_seed=種`)。

(結論・表は計測の終わりに書き足す)
