#!/bin/bash
# 今回の並び (1): mc11 と 6 問
cd "$(dirname "$0")"
python run.py mc11 def,detc 4
python run.py mc11 def_s,detc_s 3
python run.py neos-911970,physiciansched6-2,atlanta-ip,qnet1,p200x1188c,roll3000 def,detc,detrc 3
python run.py mc11 detrc,detrc_s 3
