#!/bin/sh
# 本番 2: 4 問を既定・試作 ao・alo・aow で各 3 回、d (暫定解との差) を mc11・hypothyroid-k1 で 1 回。続けて 58 問を既定・ao・alo で 1 回ずつ
cd "$(dirname "$0")"
python run.py hypothyroid-k1,neos-911970,neos-3083819-nubu,physiciansched6-2 def,r_ao,r_alo,r_aow 3
python run.py mc11,hypothyroid-k1 r_aod 1
python bench58.py def,r_ao,r_alo 0
