#!/bin/bash
# 副作用: 根のカットが効く問題で 既定 / ROOT_HSEL / ROOT_HSEL 年齢なし (各 2 回、4 スレッド 60 秒)
cd "$(dirname "$0")"
for i in 1 2; do
  for p in mzzv42z roll3000 neos-911970 beasleyC3 mc11 10teams h80x6320d misc07 qnet1 neos5 timtab1; do
    ./run_cfg.sh $p 60 def
    ./run_cfg.sh $p 60 hs ENOMOTO_MIP_ROOT_HSEL=1
    ./run_cfg.sh $p 60 hs_age0 ENOMOTO_MIP_ROOT_HSEL=1 ENOMOTO_T_MIP_HSEL_AGE=0
  done
done
