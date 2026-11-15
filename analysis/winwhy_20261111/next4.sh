#!/bin/bash
# WINWHY の続き 4: cand1112 の束 (7 項目) と、mas74 で遅めだった SYMPRE・LMIP_SWAP を 1 つだけ足した形が、木で勝つ問題を遅くするか
# (既定と対、種 1・2、120 秒。既定も同じビルドで取り直す)
D=$(cd "$(dirname "$0")" && pwd)
B=ENOMOTO_MIP_ROOTSTOP=b,ENOMOTO_MIP_RCSTRONG=irk,ENOMOTO_MIP_LMIP_SWAP=1,ENOMOTO_MIP_SYMPRE=probe,ENOMOTO_MIP_PLSB=gxu,ENOMOTO_MIP_GAP1=u,ENOMOTO_MIP_GAP4=rocn
for s in 1 2; do for p in pk1 rmatr100-p10 mas76 markshare_4_0 binkar10_1 neos5; do
  python3 $D/run.py abl $p --only ours --tag def4 --seed $s
  python3 $D/run.py abl $p --only ours --tag c1112 --env $B --seed $s
  python3 $D/run.py abl $p --only ours --tag sympre --env ENOMOTO_MIP_SYMPRE=probe --seed $s
  python3 $D/run.py abl $p --only ours --tag lmswap --env ENOMOTO_MIP_LMIP_SWAP=1 --seed $s
done; done
