#!/bin/bash
# 計測の待ち行列 1: 60 秒 × 3 回 (目的値の大きい問題・根が長くなると困る問題)、600 秒 × 2 回 (unitcal_7・mushroom-best)。ソルバーは同時に 1 つ
cd $(dirname $0)
CS="ENOMOTO_MIP_CUTSTALL_GAP=1"
for r in 1 2 3; do
  for p in neos-3083819-nubu supportcase26 rmatr100-p10 tr12-30 neos-960392 physiciansched6-2; do
    if [ $((r % 2)) = 1 ]; then ./run1.sh s60_def_r$r $p 60; ./run1.sh s60_cs_r$r $p 60 $CS
    else ./run1.sh s60_cs_r$r $p 60 $CS; ./run1.sh s60_def_r$r $p 60; fi
  done
done
for r in 1 2; do
  for p in unitcal_7 mushroom-best; do
    ./run1.sh l600_def_r$r $p 600; ./run1.sh l600_cs_r$r $p 600 $CS
  done
done
for r in 1 2; do ./run1.sh l600_csg_r$r unitcal_7 600 $CS ENOMOTO_MIP_CUTSEL_PROBE=gain; done
echo q1 done
