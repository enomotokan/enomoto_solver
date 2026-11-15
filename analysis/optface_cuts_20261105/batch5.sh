#!/bin/bash
# 利用者の案の改良: par3sg (ENOMOTO_MIP_OPTFACE_PARCUTS=3sg、下界を上げたラウンドの後に効いているカットだけを渡し、5 ラウンド目から停滞した写しだけが
# 5 本まで受け取る)。neos859080・beasleyC3 を種 1〜8・300 秒。結果は batch5.txt
cd "$(dirname "$0")"
for prob in neos859080 beasleyC3; do
  for s in 1 2 3 4 5 6 7 8; do
    ./run1.sh $prob ${prob}_par3sg_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3sg >> batch5.txt
  done
done
echo ALLDONE >> batch5.txt
