#!/bin/bash
# 6 節の 1: 根のカットの周の内訳 (ENOMOTO_MIP_DEBUG_SEPPROF=1) を、既定と ENOMOTO_MIP_SEP_PARALLEL=1 で、3 問の 4T に取る (根だけ見るので上限は短め)
cd "$(dirname "$0")"
for spec in "supportcase7 130" "mzzv42z 60" "neos-1171448 30"; do
  set -- $spec
  PROF_TAG=_sepprof PROF_ENV="ENOMOTO_MIP_DEBUG_SEPPROF=1" python3 prof.py $1 4 $2
  PROF_TAG=_seppar PROF_ENV="ENOMOTO_MIP_DEBUG_SEPPROF=1 ENOMOTO_MIP_SEP_PARALLEL=1" python3 prof.py $1 4 $2
done
echo BATCH3_DONE
