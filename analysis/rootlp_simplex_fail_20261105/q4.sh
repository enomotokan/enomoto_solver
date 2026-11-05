#!/bin/bash
# 58 問 (既定と combB) と Mittelmann 29 問の根の LP の同時実行の診断 (combB)
cd "$(dirname "$0")"
python run58.py p58c.jsonl base combB
env ENOMOTO_SIMPLEX_SCALE=1 ENOMOTO_T_SIMPLEX_SCALE_MIN_M=10000 ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE=1e4 ENOMOTO_SIMPLEX_HARRIS2=r ENOMOTO_SIMPLEX_NOISE_RESET=0.1 ENOMOTO_SIMPLEX_NOISE_RESET_W=e \
  python ../rootlp_both_diff_20261105/run.py both_combB_mitt.jsonl 150 $(cat mitt29.txt)
echo Q4_DONE
