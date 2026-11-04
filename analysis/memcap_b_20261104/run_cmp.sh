#!/bin/bash
# 設定ごとに uccase12 と CMS750_4 を同時に 600 秒 (4 スレッドずつ、この機械は 4 コア) 解く (2026-11-04、memcap_b)。
# 使い方: bash run_cmp.sh <名前>:<環境変数 (空白区切り)> ...
D=$(dirname "$0")
for spec in "$@"; do
  tag=${spec%%:*}; envs=${spec#*:}
  env MEM_TAG=$tag ENOMOTO_MIP_MEMB_DIAG=1 $envs python $D/run_mem.py 600 uccase12 &
  env MEM_TAG=$tag ENOMOTO_MIP_MEMB_DIAG=1 $envs python $D/run_mem.py 600 CMS750_4 &
  wait
done
