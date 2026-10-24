#!/bin/bash
# batch_e2pool.sh: 同値の代入 (ENOMOTO_MIP_PROBE_EQUIV2) + プールの選び直しの形 (batch_pool.sh と同じ組み合わせ) の確かめ (60 秒・4 スレッド)
cd "$(dirname "$0")"
E="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_ROOT_CUT_HIGHS=1 ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAX_CUTS=1000 ENOMOTO_MIP_CUTSEL_HIGHS=1 ENOMOTO_MIP_CUTSEL_HIGHS_POOL=1 ENOMOTO_T_MIP_ROOT_POOL_ROUNDS=1000 ENOMOTO_T_MIP_ROOT_CUT_AGE=10"
for i in 1 2 3 4; do ./run_cfg.sh physiciansched6-2 60 e2pool $E; done
for p in mzzv42z timtab1 neos-960392 app1-1 qiu h80x6320d nw04 10teams; do
  for i in 1 2; do ./run_cfg.sh $p 60 e2pool $E; done
done
