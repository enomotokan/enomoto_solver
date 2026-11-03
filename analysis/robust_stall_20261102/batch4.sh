#!/bin/sh
# 本番 4 (robust_cap の p を取り込んだ版で作り直し): roll3000・mc11 を g+p・g+o+p・o+p で各 3 回
cd "$(dirname "$0")"
python run.py roll3000,mc11 g_cp,g_o_cp,r_o_cp 3
