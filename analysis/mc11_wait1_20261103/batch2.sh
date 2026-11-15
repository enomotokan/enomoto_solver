#!/bin/bash
# mc11_wait1 の本番 2: 試作 ENOMOTO_MIP_MC11FIX1_RACE (f1s・f1i) と既定。mc11 は 4 回、5 問は 2 回
cd "$(dirname "$0")"
python run.py mc11 def,f1s,f1i 4 10
python run.py neos-911970,neos-3083819-nubu,physiciansched6-2,hypothyroid-k1,roll3000 def,f1s,f1i 2
