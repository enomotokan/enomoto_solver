#!/bin/bash
# 既定と Qf を交互に 3 回ずつ (働く条件に当たる問題と担当の問題)
D=$(cd $(dirname "$0") && pwd)
for k in 1 2 3; do
  for p in qnet1 gt2 flugpl blend2 gr4x6 rout mik-250-20-75-4 h80x6320d noswot mc11 mzzv42z 30n20b8 neos-1456979 timtab1 gen-ip002 gen-ip054 roll3000; do
    for c in def Qf; do $D/erun.sh $p $c $k; done
  done
done
