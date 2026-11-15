#!/bin/bash
# par3sg の受け取りの判定を緩めた形 (par3sgl): ENOMOTO_T_MIP_OPTFACE_SHARE_EPS=1e-4・ENOMOTO_T_MIP_OPTFACE_SHARE_START=3。neos859080・beasleyC3 を種 1〜8・300 秒。結果は batch6.txt
cd "$(dirname "$0")"
for prob in beasleyC3 neos859080; do
  for s in 1 2 3 4 5 6 7 8; do
    ./run1.sh $prob ${prob}_par3sgl_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3sg ENOMOTO_T_MIP_OPTFACE_SHARE_EPS=1e-4 ENOMOTO_T_MIP_OPTFACE_SHARE_START=3 >> batch6.txt
  done
done
echo ALLDONE >> batch6.txt
