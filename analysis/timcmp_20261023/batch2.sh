#!/bin/bash
# 60 秒: timtab1 を既定・mixoc とそれぞれに SYM_SKIP_FIXED を足した形で 3 回ずつ (交互)、対称性のある問題で副作用を 1 回ずつ
cd "$(dirname "$0")"
MIX="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
SF=ENOMOTO_MIP_SYM_SKIP_FIXED=1
for r in 1 2 3; do
  ./run.sh eno timtab1 60 b2def
  ./run.sh eno timtab1 60 b2sf $SF
  ./run.sh eno timtab1 60 b2mixoc $MIX
  ./run.sh eno timtab1 60 b2mixocsf $MIX $SF
done
for p in roll3000 neos-960392 physiciansched6-2 beasleyC3; do
  ./run.sh eno $p 60 b2def
  ./run.sh eno $p 60 b2sf $SF
  ./run.sh eno $p 60 b2mixoc $MIX
  ./run.sh eno $p 60 b2mixocsf $MIX $SF
done
echo BATCH2_DONE
