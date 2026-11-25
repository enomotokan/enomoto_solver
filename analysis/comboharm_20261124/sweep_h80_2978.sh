#!/bin/bash
# h80x6320d・neos-2978193-inde の切り分け (1 スレッド、種 1、600 秒、1 本ずつ)。出力 res_h80.jsonl・res_2978.jsonl、2 回目は *_rep2.jsonl
cd "$(dirname "$0")/../.."
R=analysis/comboharm_20261124/run.py; A=analysis/comboharm_20261124
C=def,combo,-sbpb,-cutu,-bfrt,-ppi,-heur,-nra,-enum,-tie
for suf in "" _rep2; do
  python $R $A/res_h80$suf.jsonl h80x6320d 1 $C
  CH_EXTRA="ENOMOTO_MIP_NODEROWS_ADAPT=tk ENOMOTO_MIP_ENUM_ADAPT=1s" python $R $A/res_h80_ks$suf.jsonl h80x6320d 1 combo
  python $R $A/res_2978$suf.jsonl neos-2978193-inde 1 $C
  CH_EXTRA="ENOMOTO_MIP_NODEROWS_ADAPT=tk ENOMOTO_MIP_ENUM_ADAPT=1s" python $R $A/res_2978_ks$suf.jsonl neos-2978193-inde 1 combo
done
