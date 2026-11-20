#!/bin/bash
# unitcal_7 の根だけ: カットの選び方・族の切り替え
D=$(cd "$(dirname "$0")" && pwd)
S=ENOMOTO_MIP_DEBUG_SEPPROF=1
python "$D/root.py" unitcal_7 probe0 ENOMOTO_MIP_CUTSEL_PROBE=0 $S
python "$D/root.py" unitcal_7 hcm ENOMOTO_MIP_HIGHS_CUTMGMT=1 $S
python "$D/root.py" unitcal_7 zh ENOMOTO_MIP_NO_ZEROHALF=0 $S
python "$D/root.py" unitcal_7 mc2000 ENOMOTO_T_MIP_ROOT_MAX_CUTS=2000 $S
