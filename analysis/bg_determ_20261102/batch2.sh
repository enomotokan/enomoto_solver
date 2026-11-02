#!/bin/bash
# 今回の並び (2): 窓の判定の保留を入れた版 (札 v2、土台にもう一方のセッションの LP_WORK を含む)
cd "$(dirname "$0")"
export BGD_TAG=v2
python run.py mc11 def,detc 4 1
python run.py mc11 def_s,detc_s 3
python run.py neos-911970,physiciansched6-2,atlanta-ip,qnet1,p200x1188c,roll3000 def,detc,detrc 3
python run.py mc11,physiciansched6-2 lw,detc_lw 3
