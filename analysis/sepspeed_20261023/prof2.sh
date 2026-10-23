#!/bin/bash
# 試作の効果 (1 スレッド・根だけ): 既定 (substitute の高速化込み) / FRACSKIP / FRACSKIP + FACE_ADAPT
cd $(dirname $0)
B="NODE_LIMIT=1 ENOMOTO_MIP_NO_BG_CUTS=1 ENOMOTO_MIP_DEBUG_SEPPROF=1 ENOMOTO_MIP_DEBUG_FACETIME=1"
for P in roll3000 mzzv42z 30n20b8 neos-911970 physiciansched6-2; do
  ./run_eno.sh $P ${P}_t1_sub 60 1 $B
  ./run_eno.sh $P ${P}_t1_fs 60 1 $B ENOMOTO_MIP_GEN_FRACSKIP=1
  ./run_eno.sh $P ${P}_t1_fsfa 60 1 $B ENOMOTO_MIP_GEN_FRACSKIP=1 ENOMOTO_MIP_FACE_ADAPT=1
done
