#!/bin/bash
# 利用者の案 (途中まで別々に分離し、一番下界が高かった写しだけを、写し全部の分のスレッドで既定のように続ける): par3k (ENOMOTO_MIP_OPTFACE_PARCUTS=3k)。
# beasleyC3・neos859080 を種 1〜8・300 秒 (今のビルド)。結果は batch10.txt
cd "$(dirname "$0")"
for prob in beasleyC3 neos859080; do
  for s in 1 2 3 4 5 6 7 8; do
    ./run1.sh $prob ${prob}_par3k_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3k >> batch10.txt
  done
done
echo ALLDONE >> batch10.txt
