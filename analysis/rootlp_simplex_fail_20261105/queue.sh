#!/bin/bash
# 計測の順: 9 問の根の LP (組み合わせ)、58 問 (既定と組み合わせ)、Mittelmann 29 問の根の LP の競争の診断 (組み合わせ)
cd "$(dirname "$0")"
COMB="ENOMOTO_SIMPLEX_SCALE=1 ENOMOTO_T_SIMPLEX_SCALE_MIN_M=10000 ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE=1e4 ENOMOTO_SIMPLEX_HARRIS2=r"
env $COMB python run.py comb.jsonl 150 thor50dday sing326 sing44 uccase9 snp-02-004-104 uccase12 neos-873061 rail01
python run58.py p58b.jsonl base comb
env $COMB python ../rootlp_both_diff_20261105/run.py both_comb_mitt.jsonl 150 $(cat mitt29.txt)
echo QUEUE_DONE
