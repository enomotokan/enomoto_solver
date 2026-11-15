#!/bin/bash
# batch1 の後: neos859080 を OPTFACE_MOVE=d (離れた頂点) で種 1〜8 (300 秒)。結果の 1 行は batch2.txt
cd "$(dirname "$0")"
until grep -q ALLDONE batch1.txt; do sleep 10; done
for s in 1 2 3 4 5 6 7 8; do
  ./run1.sh neos859080 neos859080_mvd_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_MOVE=d >> batch2.txt
done
echo ALLDONE >> batch2.txt
