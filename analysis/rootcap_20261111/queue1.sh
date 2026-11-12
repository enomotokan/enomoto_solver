#!/bin/sh
# ROOTCAP: 対象 5 問を 600 秒・種 1 で、hd・def・h・HiGHS の順に 1 つずつ回す (ソルバーは同時に 1 つ)
cd "$(dirname "$0")"
for cfg in hd def h highs; do
  for p in neos-873061 neos-3216931-puriri unitcal_7 net12 neos-4722843-widden; do
    python3 run1.py $p $cfg 1 600
  done
done
