#!/bin/bash
# 改訂 2 (根は外側の始まりから 10 秒・木は 15 秒たってから) 後: v3 の名前で。ACS の違反行の近くの固定の 2 回目も
cd "$(dirname "$0")"; source configs.sh
CFG[lbprox]="ENOMOTO_MIP_LB2=1 ENOMOTO_MIP_PROX2=1"
CFG[acsviol]="ENOMOTO_MIP_ACS=1 ENOMOTO_MIP_ACS_FIX=viol"
for r in 1 2; do
  for p in misc07 10teams mik-250-20-75-4 qnet1; do for c in lbprox all5; do ./run_cfg.sh $p 60 ${c}v3 ${CFG[$c]}; done; done
  [ $r = 2 ] && for p in 30n20b8 neos-1456979; do ./run_cfg.sh $p 60 acsviol ${CFG[acsviol]}; done
  for p in mzzv42z physiciansched6-2 supportcase26 roll3000; do for c in lb2 prox2 lbprox; do ./run_cfg.sh $p 60 ${c}v3 ${CFG[$c]}; done; done
done
