#!/bin/bash
# physiciansched3-3 を 300 秒で既定と LB2+PROX2 (v3) を 1 回ずつ (60 秒では根の LP が 51 秒かかり暫定解が出ない)
cd "$(dirname "$0")"
./run_cfg.sh physiciansched3-3 300 base300
./run_cfg.sh physiciansched3-3 300 lbprox300 ENOMOTO_MIP_LB2=1 ENOMOTO_MIP_PROX2=1
