#!/bin/bash
# 確かめ 1 (README 1 節): 既定と既定 + ENOMOTO_MIP_LMIP_SCORE_TICKS=1 を、1 スレッド・種 1・120 秒で 1 本ずつ、2 回。
# リポジトリの根から: bash analysis/lmipticks_20261125/t1.sh  → res_t1_{def,sct}{,_rep2}.jsonl
cd "$(dirname "$0")/../.."
D=analysis/lmipticks_20261125; R=analysis/comboharm_20261124/run.py
PS="fhnw-binpack4-48 fhnw-binpack4-4 ns1116954 30n20b8 neos-4763324-toguru chromaticindex512-7 neos8 app1-1 traininstance6 traininstance2 rocII-5-11 supportcase42 neos-5107597-kakapo istanbul-no-cutoff chromaticindex1024-7"
for rep in "" _rep2; do
  for p in $PS; do
    python $R $D/res_t1_def$rep.jsonl $p 1 def --tl 120
    CH_EXTRA="ENOMOTO_MIP_LMIP_SCORE_TICKS=1" python $R $D/res_t1_sct$rep.jsonl $p 1 def --tl 120
  done
done
