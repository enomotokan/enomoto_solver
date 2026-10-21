#!/bin/bash
# usage: loop.sh tag inst N threads tl [env...]
tag=$1; inst=$2; N=$3; th=$4; tl=$5; shift 5
D=/tmp/claude-0/-home-user-enomoto-solver/c87a0e23-38ed-50e9-aee1-579171b6da25/scratchpad
for i in $(seq 1 $N); do
  env "$@" ENOMOTO_MIP_LOG=1 python3 $D/run1.py /home/user/enomoto_solver/.miplib_cache/$inst.mps.gz $th $tl > $D/runs/$tag.$i.log 2>&1
  echo "$i $(grep RESULT $D/runs/$tag.$i.log)" >> $D/runs/$tag.summary
done
