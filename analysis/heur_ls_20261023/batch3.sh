#!/bin/bash
# fl = FJ のスレッドで FJ が 2 回続けて解なしなら Local-MIP + 連鎖 (ENOMOTO_MIP_FJ_LOCALMIP)
# s1 = 根のカットの生成のスレッドをスレッド数 - 1 (ENOMOTO_MIP_SEP_THREADS_M1)、fls = 両方。def は batch1 のもの
cd "$(dirname "$0")"
for rep in 1 2; do
for p in 30n20b8 neos-1456979 physiciansched6-2 roll3000 mzzv42z timtab1; do
  ./run_cfg.sh $p 60 fl ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_LOCALMIP=1
  ./run_cfg.sh $p 60 s1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_SEP_THREADS_M1=1
  ./run_cfg.sh $p 60 fls ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_LOCALMIP=1 ENOMOTO_MIP_SEP_THREADS_M1=1
done
done
echo BATCH3 DONE
