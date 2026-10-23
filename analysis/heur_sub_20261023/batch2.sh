#!/bin/bash
# 暫定解の改善が遅い問題で Local Branching 2・Proximity 2 (2 回ずつ)
cd "$(dirname "$0")"; source configs.sh
CFG[lbprox]="ENOMOTO_MIP_LB2=1 ENOMOTO_MIP_PROX2=1"
for r in 1 2; do for p in mzzv42z physiciansched6-2 supportcase26 roll3000; do for c in base lb2 prox2 lbprox; do
  ./run_cfg.sh $p 60 $c ${CFG[$c]}
done; done; done
