#!/bin/bash
# 副作用の確認: 判定に当たる問題を既定と FC_CUT_ROUNDS で 2 回ずつ (順番を入れ替える)
cd /home/user/enomoto_solver/runs_mc11
for p in "$@"; do
  ./gp.sh $p def_a 60 ENOMOTO_MIP_LOG=1
  ./gp.sh $p fcr_a 60 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FC_CUT_ROUNDS=1
  ./gp.sh $p fcr_b 60 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FC_CUT_ROUNDS=1
  ./gp.sh $p def_b 60 ENOMOTO_MIP_LOG=1
done
