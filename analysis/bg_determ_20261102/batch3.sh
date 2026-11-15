#!/bin/bash
# 今回の並び (3): 既定の待ちの上限を縮めた形 (遅い機械の真似)、mc11 の rc、58 問 (既定・detc 1 回ずつ)
cd "$(dirname "$0")"
export BGD_TAG=v2
python run.py mc11 def_m1,def_s_m1,detc_s_m1 3
python run.py mc11 detrc,detrc_s 2
python bench58.py def,detc 0
