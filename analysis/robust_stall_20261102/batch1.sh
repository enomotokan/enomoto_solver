#!/bin/sh
# 本番 1: roll3000・mc11 を 既定・DSE=g と、それぞれに試作 ao・alo を足した形で各 3 回
cd "$(dirname "$0")"
python run.py roll3000,mc11 def,g,r_ao,g_ao,r_alo,g_alo 3
