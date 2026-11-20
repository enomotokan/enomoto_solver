#!/bin/bash
# mzzv42z の回し直し (時刻つき) と unitcal_7 の根だけの比べ
D=$(cd "$(dirname "$0")" && pwd)
"$D/one.sh" mzzv42z ours 1 600 def
"$D/one.sh" mzzv42z highs 1 600 highs
python "$D/root.py" unitcal_7 def ENOMOTO_MIP_DEBUG_SEPPROF=1
python "$D/root.py" unitcal_7 r100 ENOMOTO_T_MIP_CUT_ROUNDS=100 ENOMOTO_MIP_DEBUG_SEPPROF=1
python "$D/root.py" unitcal_7 p2 ENOMOTO_MIP_CUT_PHASE2=1 ENOMOTO_MIP_DEBUG_SEPPROF=1
