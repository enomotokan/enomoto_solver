#!/bin/bash
# WINWHY の続き 5: cand1112 の束で pk1・rmatr100-p10 が 2 種とも遅い。SYMPRE・LMIP_SWAP 以外の 5 項目を 1 つずつ (種 1・2、120 秒)
D=$(cd "$(dirname "$0")" && pwd)
for s in 1 2; do for p in pk1 rmatr100-p10; do
  python3 $D/run.py abl $p --only ours --tag rootstop --env ENOMOTO_MIP_ROOTSTOP=b --seed $s
  python3 $D/run.py abl $p --only ours --tag rcstrong --env ENOMOTO_MIP_RCSTRONG=irk --seed $s
  python3 $D/run.py abl $p --only ours --tag plsb --env ENOMOTO_MIP_PLSB=gxu --seed $s
  python3 $D/run.py abl $p --only ours --tag gap1 --env ENOMOTO_MIP_GAP1=u --seed $s
  python3 $D/run.py abl $p --only ours --tag gap4 --env ENOMOTO_MIP_GAP4=rocn --seed $s
done; done
