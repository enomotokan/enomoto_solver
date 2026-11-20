#!/bin/bash
# VB_IMPL + IMPLBD_SELECT だけ (vbsel)、unitcal_7 種 1・2、600 秒
D=$(cd "$(dirname "$0")" && pwd)
for s in 1 2; do "$D/one.sh" unitcal_7 ours $s 600 vbsel ENOMOTO_MIP_VB_IMPL=1 ENOMOTO_MIP_IMPLBD_SELECT=1; done
