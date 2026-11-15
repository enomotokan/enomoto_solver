#!/bin/bash
# par3m の続きを裏のカットと同じ止め方 (手間の上限で停滞まで) にした形 (par3m2、同じ ENOMOTO_MIP_OPTFACE_PARCUTS=3m)。beasleyC3・neos859080 を種 1〜8・300 秒。結果は batch9.txt
cd "$(dirname "$0")"
for prob in beasleyC3 neos859080; do
  for s in 1 2 3 4 5 6 7 8; do
    ./run1.sh $prob ${prob}_par3m2_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3m >> batch9.txt
  done
done
echo ALLDONE >> batch9.txt
