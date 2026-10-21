#!/bin/bash
cd "$(dirname "$0")"
./run.sh base300_4 300
./run.sh fr300_4 300 ENOMOTO_MIP_FLAT_RESTART=1
./run.sh fr300_5 300 ENOMOTO_MIP_FLAT_RESTART=1
./run.sh base300_5 300
python runh.py ../../.miplib_cache/neos-2978193-inde.mps 60 > logs/highs_60_2.log 2>&1; echo "highs_60_2 $(grep RESULT logs/highs_60_2.log)"
