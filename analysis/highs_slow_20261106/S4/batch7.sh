#!/bin/bash
# neos-957323 の木の試作 DEGSCORE (TINYGAP=1 で木に入れて比べる)
cd "$(dirname "$0")"
python3 run1.py tg1dg_s0 neos-957323 ours 0 ENOMOTO_MIP_S4_TINYGAP=1 ENOMOTO_MIP_S4_DEGSCORE=1
python3 run1.py dg_s0 neos-957323 ours 0 ENOMOTO_MIP_S4_DEGSCORE=1
