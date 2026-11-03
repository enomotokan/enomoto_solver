#!/bin/sh
# 本番 3: o だけ (窓の判定はそのまま、ループの今までの停滞の判定を窓の判定で通す) を roll3000・mc11 (既定・DSE=g) と 4 問で各 3 回、58 問で 1 回
cd "$(dirname "$0")"
python run.py roll3000,mc11 r_o,g_o 3
python run.py hypothyroid-k1,neos-911970,neos-3083819-nubu,physiciansched6-2 r_o 3
python bench58.py r_o 0
