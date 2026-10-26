#!/bin/bash
# batch_pool.sh: 根のカットのプールを毎ラウンド HiGHS の点数で選び直す形 (pool) の確かめ (60 秒・4 スレッド)。既存の切り替えの組み合わせ:
# ENOMOTO_MIP_ROOT_CUT_HIGHS (時間の上限・LP の上限) + 1 ラウンド 1000 本 + ENOMOTO_MIP_CUTSEL_HIGHS (違反量 / (効いている非零 x ノルム)、
# 最良の点数の割合で切る、平行度 0.1) + ENOMOTO_MIP_CUTSEL_HIGHS_POOL (プールで違反しているカットを毎ラウンド候補に戻す) + LP のカットの年齢 10
cd "$(dirname "$0")"
E="ENOMOTO_MIP_ROOT_CUT_HIGHS=1 ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAX_CUTS=1000 ENOMOTO_MIP_CUTSEL_HIGHS=1 ENOMOTO_MIP_CUTSEL_HIGHS_POOL=1 ENOMOTO_T_MIP_ROOT_POOL_ROUNDS=1000 ENOMOTO_T_MIP_ROOT_CUT_AGE=10"
for i in 1 2 3; do ./run_cfg.sh physiciansched6-2 60 pool $E; done
for p in mzzv42z timtab1 neos-960392 app1-1 qiu h80x6320d nw04 10teams; do
  for i in 1 2; do ./run_cfg.sh $p 60 pool $E; done
done
