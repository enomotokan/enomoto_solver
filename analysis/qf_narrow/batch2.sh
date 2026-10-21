#!/bin/bash
# 切り分け: qnet1 (G/R/F/X)、h80x6320d・noswot の追加の回、mc11・mzzv42z の X なし
D=$(cd $(dirname "$0") && pwd)
for k in 1 2 3; do
  for c in QfnoX X G R F GR GRF; do $D/erun.sh qnet1 $c $k; done
done
for k in 4 5 6; do
  for p in h80x6320d noswot rout; do for c in def Qf QfnoX; do $D/erun.sh $p $c $k; done; done
done
for k in 1 2 3; do
  for p in h80x6320d; do $D/erun.sh $p X $k; done
  for p in mc11 mzzv42z; do $D/erun.sh $p QfnoX $k; done
done
