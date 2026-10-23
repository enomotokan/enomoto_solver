#!/bin/bash
# physiciansched3-3 (MIPLIB から取得) を既定と LB2+PROX2 (v3) で 2 回ずつ
cd "$(dirname "$0")"
for r in 1 2; do
  ./run_cfg.sh physiciansched3-3 60 base
  ./run_cfg.sh physiciansched3-3 60 lbproxv3 ENOMOTO_MIP_LB2=1 ENOMOTO_MIP_PROX2=1
done
