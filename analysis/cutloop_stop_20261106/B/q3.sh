#!/bin/bash
# 計測の待ち行列 3 (q2 の後): 直したビルドの cs (名前 cs2) で 60 秒 6 問 × 3 回、600 秒 unitcal_7・mushroom-best × 2 回。既定は q1 の回を使う
cd $(dirname $0)
until grep -q "q2 done" q2.log; do sleep 30; done
CS="ENOMOTO_MIP_CUTSTALL_GAP=1"
for r in 1 2 3; do
  for p in neos-3083819-nubu supportcase26 rmatr100-p10 tr12-30 neos-960392 physiciansched6-2; do ./run1.sh s60_cs2_r$r $p 60 $CS; done
done
for r in 1 2; do for p in unitcal_7 mushroom-best; do ./run1.sh l600_cs2_r$r $p 600 $CS; done; done
echo q3 done
