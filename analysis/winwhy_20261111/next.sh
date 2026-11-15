#!/bin/bash
# WINWHY の続き: (1) 試作 ENOMOTO_MIP_CLQEXT_KNAP の当たりの下見 (91 問、20 秒、種 1)、(2) DRCR の規則を外す形の種 3・4、(3) 600 秒の組
D=$(cd "$(dirname "$0")" && pwd)
python3 $D/run.py scr --only ours --tag knap --env ENOMOTO_MIP_CLQEXT_KNAP=1
for p in cbs-cta ns1208400; do for s in 3 4; do
  python3 $D/run.py abl $p --only ours --tag def --seed $s
  python3 $D/run.py abl $p --only ours --tag foldp --env ENOMOTO_MIP_FOLD=rip --seed $s
done; done
python3 $D/run.py o600
