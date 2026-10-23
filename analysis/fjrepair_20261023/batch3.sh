#!/bin/bash
# 3. 既定 (def、環境変数なし) と試作 (fjr = ENOMOTO_MIP_FJ_REPAIR=1 + 診断 FJ_STATS) を 2 回ずつ (4 スレッド・60 秒)
cd "$(dirname "$0")"
P="30n20b8 neos-1456979 neos5 physiciansched6-2 supportcase26 graph20-20-1rand timtab1 mc11 roll3000 beasleyC3 air03 blend2"
for r in 1 2; do
  for p in $P; do
    [ -e logs/${p}_def_${r}.log ] || ./run_cfg.sh $p 60 def
    [ -e logs/${p}_fjr3_${r}.log ] || ./run_cfg.sh $p 60 fjr3 ENOMOTO_MIP_FJ_STATS=1 ENOMOTO_MIP_FJ_REPAIR=1
  done
done
echo ALL DONE
