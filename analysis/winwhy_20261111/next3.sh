#!/bin/bash
# WINWHY の続き 3: HiGHS に近づける変更 (cand1111 の CUTCONF・TREEFIX) が、木で勝つ問題の強みを壊すか (種 1・2、120 秒)
D=$(cd "$(dirname "$0")" && pwd)
for p in pk1 rmatr100-p10 markshare_4_0 mas76; do for s in 1 2; do
  python3 $D/run.py abl $p --only ours --tag cutconf --env ENOMOTO_MIP_CUTCONF=1 --seed $s
  python3 $D/run.py abl $p --only ours --tag treefix --env ENOMOTO_MIP_TREEFIX=pcu --seed $s
done; done
