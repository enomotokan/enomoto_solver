#!/bin/bash
# neos859080・beasleyC3 (300 秒)、n2seq36q (120 秒) を 4 スレッド・種 1〜8 で、5 設定を交互に。結果の 1 行は batch1.txt
cd "$(dirname "$0")"
declare -A CFG=( [def]="" [rob]="ENOMOTO_MIP_ROBUST_ROOTIPM=1" [both]="ENOMOTO_MIP_ROOTLP_BOTH=1" [mv]="ENOMOTO_MIP_OPTFACE_MOVE=f" [par]="ENOMOTO_MIP_OPTFACE_PARCUTS=3" )
for prob in neos859080 beasleyC3 n2seq36q; do
  tl=300; [ $prob = n2seq36q ] && tl=120
  for s in 1 2 3 4 5 6 7 8; do
    for c in def rob both mv par; do
      ./run1.sh $prob ${prob}_${c}_s$s 4 $s $tl ${CFG[$c]} >> batch1.txt
    done
  done
done
echo ALLDONE >> batch1.txt
