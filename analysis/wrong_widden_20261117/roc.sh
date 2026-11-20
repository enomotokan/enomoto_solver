#!/bin/bash
# rocI-4-11 の時間切れの切り分け: 既定 / LPINF なし / PROP_RELTOL_CONT=0 を種 1・2、600 秒 (ソルバーは同時に 1 つ)
D=$(cd "$(dirname "$0")" && pwd)
for s in 1 2; do
  python "$D/one.py" rocI-4-11 def $s 600
  python "$D/one.py" rocI-4-11 nolpinf $s 600 ENOMOTO_MIP_NO_LPINF_CHECK=1
  python "$D/one.py" rocI-4-11 rtc0 $s 600 ENOMOTO_T_PROP_RELTOL_CONT=0
done
