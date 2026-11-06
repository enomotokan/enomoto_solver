#!/bin/bash
# 当たりをつける試し (既存のつまみ・試作)、各 1〜2 回
cd "$(dirname "$0")"
r() { python3 run1.py ours "$@"; }
r neos-860300 strongfrac01 1 300 ENOMOTO_T_MIP_CUT_STRONG_FRAC=0.1
r neos-860300 strongfrac01 2 300 ENOMOTO_T_MIP_CUT_STRONG_FRAC=0.1
r neos-860300 p2cap 1 300 ENOMOTO_MIP_CUT_PHASE2_AT_CAP=1
r neos-860300 rounds60 1 300 ENOMOTO_T_MIP_CUT_ROUNDS=60
r hypothyroid-k1 rounds5 1 300 ENOMOTO_T_MIP_CUT_ROUNDS=5
r hypothyroid-k1 rounds5 2 300 ENOMOTO_T_MIP_CUT_ROUNDS=5
r physiciansched6-2 roleearly 1 300 ENOMOTO_MIP_HO1_ROLEEARLY=1
r physiciansched6-2 stallrw 1 300 ENOMOTO_MIP_ROBUST_STALL=rw
r physiciansched6-2 rootlpboth 1 300 ENOMOTO_MIP_ROOTLP_BOTH=1
echo ALL_DONE
