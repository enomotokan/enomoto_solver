#!/bin/bash
# CLQPRB: 対象の問題 × 設定で、根の LP の前の大きさと根の LP を取る (epre.py)。1 つずつ順に回す。
cd "$(dirname "$0")"
P="${P:-mzzv11 mzzv42z neos-3216931-puriri physiciansched6-2}"
for p in $P; do
  python3 epre.py def $p
  python3 epre.py c1 $p ENOMOTO_MIP_CLQPRB=1
  python3 epre.py c1k $p ENOMOTO_MIP_CLQPRB=1 ENOMOTO_MIP_KEEP_IMPLICATIONS=1
done
