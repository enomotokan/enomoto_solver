#!/bin/bash
# neos859080・beasleyC3 (4 スレッド・300 秒、種 1〜10) と n2seq36q (4 スレッド・300 秒、種 1〜3) を def・rob・both で交互に
cd "$(dirname "$0")"
for s in 1 2 3 4 5 6 7 8 9 10; do
  for p in neos859080 beasleyC3; do
    ./run1.sh $p ${p}_def_s$s 4 $s 300
    ./run1.sh $p ${p}_rob_s$s 4 $s 300 ENOMOTO_MIP_ROBUST_ROOTIPM=1
    ./run1.sh $p ${p}_both_s$s 4 $s 300 ENOMOTO_MIP_ROOTLP_BOTH=1
  done
done >> batch1.txt
for s in 1 2 3; do
  p=n2seq36q
  ./run1.sh $p ${p}_def_s$s 4 $s 300
  ./run1.sh $p ${p}_rob_s$s 4 $s 300 ENOMOTO_MIP_ROBUST_ROOTIPM=1
  ./run1.sh $p ${p}_both_s$s 4 $s 300 ENOMOTO_MIP_ROOTLP_BOTH=1
done >> batch1.txt
python bench58.py def,both 1
python bench58.py def,both 2
echo ALLDONE >> batch1.txt
