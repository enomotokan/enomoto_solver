#!/bin/bash
# 3 案の組み合わせ A・B: 9 問 (二段解法だけ) と、supportcase42 の同時実行の診断
cd "$(dirname "$0")"
A="ENOMOTO_SIMPLEX_HARRIS2=t ENOMOTO_T_XB_CONSISTENCY_TOL=0 ENOMOTO_SIMPLEX_NOISE_RESET=0.1"
B="ENOMOTO_SIMPLEX_HARRIS2=r ENOMOTO_SIMPLEX_NOISE_RESET=0.1 ENOMOTO_SIMPLEX_NOISE_RESET_W=e"
SC="ENOMOTO_SIMPLEX_SCALE=1 ENOMOTO_T_SIMPLEX_SCALE_MIN_M=10000 ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE=1e4"
P="supportcase42 thor50dday sing326 sing44 uccase9 snp-02-004-104 uccase12 neos-873061 rail01"
env $A python run.py fixA.jsonl 150 $P
env $B python run.py fixB.jsonl 150 $P
env $A $SC python ../rootlp_both_diff_20261105/run.py both_fixA.jsonl 150 supportcase42
env $B $SC python ../rootlp_both_diff_20261105/run.py both_fixB.jsonl 150 supportcase42
echo Q3_DONE
