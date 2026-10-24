#!/bin/bash
# (c) 40 問 + 16 問を 20 秒・4 スレッドで 1 回ずつ (ENOMOTO_MIP_LOG 付き)。既定 (c_def) と手当て (c_<NAME>)
cd "$(dirname "$0")"
H=${H:-1}; name=${NAME:-h1}
P40=$(python3 -c "import sys; sys.path.insert(0,'../../scripts'); from run_miplib_benchmark import PROBLEMS; print(' '.join(PROBLEMS))")
P16="nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 uccase12 neos-2978193-inde neos-911970 mzzv42z neos-960392 roll3000 app1-1"
for p in $P40 $P16; do
  [ -f ../../.miplib_cache/$p.mps ] || { echo "$p no mps"; continue; }
  [ -n "$ONLY_H" ] || ./run_cfg.sh $p 20 c_def
  ./run_cfg.sh $p 20 c_$name ENOMOTO_MIP_SUBMIP_HIGHS=$H
done | tee -a logs/batch_c.txt
echo BATCH_C DONE
