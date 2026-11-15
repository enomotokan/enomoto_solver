#!/bin/bash
# 利用者の案 2 つ: first3 (ENOMOTO_MIP_OPTFACE_FIRST=3、最初のラウンドだけ 3 つの頂点で分離して足し、あとは既定のループ) と
# par3s (ENOMOTO_MIP_OPTFACE_PARCUTS=3s、写し 3 本がカットを 2 ラウンドごとに渡し合う)。neos859080・beasleyC3 を種 1〜8・300 秒。結果は batch4.txt
cd "$(dirname "$0")"
declare -A CFG=( [first3]="ENOMOTO_MIP_OPTFACE_FIRST=3" [par3s]="ENOMOTO_MIP_OPTFACE_PARCUTS=3s" )
for prob in neos859080 beasleyC3; do
  for s in 1 2 3 4 5 6 7 8; do
    for c in first3 par3s; do
      ./run1.sh $prob ${prob}_${c}_s$s 4 $s 300 ${CFG[$c]} >> batch4.txt
    done
  done
done
echo ALLDONE >> batch4.txt
