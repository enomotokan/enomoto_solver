#!/bin/sh
# mc11: def,f_a,f_b,f_c,f_ac,f_c10 × 4
cd "$(dirname "$0")"
export MW_EXTRA_CFGS='{"f_c10": {"ENOMOTO_MIP_MC11FIX2": "c", "ENOMOTO_T_MIP_MC11FIX2_P2_ROUNDS": "10"}}'
python run.py mc11 def,f_a,f_b,f_c,f_ac,f_c10 4
