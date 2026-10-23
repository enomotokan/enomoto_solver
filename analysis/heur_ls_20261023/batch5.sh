#!/bin/bash
# FJ と Local-MIP + 連鎖を単独で比べる (ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=3: 各 3 秒、根の前に主のスレッドで)。40 問 + 16 問
cd "$(dirname "$0")"
P40=$(python3 -c "import sys; sys.path.insert(0,'../../scripts'); from run_miplib_benchmark import PROBLEMS; print(' '.join(PROBLEMS))")
P16="nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 uccase12 neos-2978193-inde neos-911970 mzzv42z neos-960392 roll3000 app1-1"
for p in $P40 $P16; do
  [ -f ../../.miplib_cache/$p.mps ] || { echo "$p no mps"; continue; }
  ls logs/${p}_vs_*.log >/dev/null 2>&1 && continue
  ./run_cfg.sh $p 12 vs ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=3 > /dev/null
  echo "$p $(grep -h FJVSLM logs/${p}_vs_1.log)"
done
echo BATCH5 DONE
