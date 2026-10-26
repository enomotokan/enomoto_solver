#!/bin/bash
# 絞った試作 P1 (Qf から X を外す)・P2 (P1 + 固定費用型の F は 5000 列まで) の確認
D=$(cd $(dirname "$0") && pwd)
for k in 1 2 3; do
  for p in mc11 mzzv42z h80x6320d; do $D/erun.sh $p P2 $k; done
  for p in mik-250-20-75-4 blend2 gt2 roll3000 timtab1 30n20b8 neos-1456979 gen-ip002 gen-ip054; do $D/erun.sh $p QfnoX $k; done
  for p in nursesched-sprint02 hypothyroid-k1 neos-950242; do for c in def P1; do $D/erun.sh $p $c $k; done; done
done
