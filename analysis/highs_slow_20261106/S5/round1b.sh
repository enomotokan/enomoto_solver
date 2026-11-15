#!/bin/bash
# round1 の残り (06:58 に止まった分) と round2
cd "$(dirname "$0")"
python3 run1.py highs pg5_34 h1 600
python3 run1.py ours pg5_34 def_s2 600 ENOMOTO_T_MIP_SEED=2
python3 run1.py highs pg5_34 h2 600
