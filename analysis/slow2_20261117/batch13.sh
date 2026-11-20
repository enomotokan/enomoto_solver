#!/bin/bash
# unitcal_7 の根だけ (node_limit 1) を種 1〜4 で: 既定・vb・sel・vbis と vbis から 1 つずつ外した形。物差しは根の下界
D=$(cd "$(dirname "$0")" && pwd)
while pgrep -f batch12.sh >/dev/null; do sleep 20; done
declare -A P=([VB]="ENOMOTO_MIP_VB_IMPL=1" [IS]="ENOMOTO_MIP_IMPLBD_SELECT=1" [MR]="ENOMOTO_T_CUTSEL_MIN_REL=0.1" [MC]="ENOMOTO_T_MIP_ROOT_MAX_CUTS=200" [RD]="ENOMOTO_T_MIP_CUT_ROUNDS=60" [P2]="ENOMOTO_MIP_CUT_PHASE2=1")
ALL="${P[VB]} ${P[IS]} ${P[MR]} ${P[MC]} ${P[RD]} ${P[P2]}"
SEL="${P[IS]} ${P[MR]} ${P[MC]} ${P[RD]} ${P[P2]}"
for s in 1 2 3 4; do
  python "$D/root.py" unitcal_7 r4def_s$s ENOMOTO_T_MIP_SEED=$s
  python "$D/root.py" unitcal_7 r4vb_s$s ENOMOTO_T_MIP_SEED=$s ${P[VB]}
  python "$D/root.py" unitcal_7 r4vbis_s$s ENOMOTO_T_MIP_SEED=$s $ALL
  python "$D/root.py" unitcal_7 r4sel_s$s ENOMOTO_T_MIP_SEED=$s $SEL
  for k in IS MR MC RD P2; do python "$D/root.py" unitcal_7 r4vbis-${k}_s$s ENOMOTO_T_MIP_SEED=$s ${ALL/${P[$k]}/}; done
done
