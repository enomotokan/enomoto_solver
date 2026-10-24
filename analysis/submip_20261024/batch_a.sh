#!/bin/bash
# (a) physiciansched6-2 の根の RINS のサブ MIP の書き出し 3 つ (analysis/physprimal_20261024/subgz) を、根のサブ MIP と同じ形
# (ENOMOTO_MIP_DEBUG_AS_SUBMIP=1、1 スレッド、打ち切り値 = 外の暫定解) で 20 秒ずつ解く。手当てなし・SUBMIP_CUT_AGG・SUBMIP_HIGHS とその部分
cd "$(dirname "$0")"
S=../../.subcache
declare -A CUT=([d1_3]=50425 [d2_3]=50425 [av_3]=49824)
cfgs=${CFGS:-"base agg h1 hp hc hs hh hf hg"}
for d in d1_3 d2_3 av_3; do
  for c in $cfgs; do
    case $c in
      base) e="";; agg) e="ENOMOTO_MIP_SUBMIP_CUT_AGG=1";; h1) e="ENOMOTO_MIP_SUBMIP_HIGHS=1";;
      hp) e="ENOMOTO_MIP_SUBMIP_HIGHS=p";; hc) e="ENOMOTO_MIP_SUBMIP_HIGHS=c";; hs) e="ENOMOTO_MIP_SUBMIP_HIGHS=s";;
      hh) e="ENOMOTO_MIP_SUBMIP_HIGHS=h";; hf) e="ENOMOTO_MIP_SUBMIP_HIGHS=f";; hg) e="ENOMOTO_MIP_SUBMIP_HIGHS=g";;
      hpc) e="ENOMOTO_MIP_SUBMIP_HIGHS=pc";; hcs) e="ENOMOTO_MIP_SUBMIP_HIGHS=cs";; hch) e="ENOMOTO_MIP_SUBMIP_HIGHS=ch";;
    esac
    TH=1 ./run_cfg.sh $S/$d.mps 20 s_$c ENOMOTO_MIP_DEBUG_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=${CUT[$d]} $e
  done
done | tee -a logs/batch_a.txt
echo BATCH_A DONE
