#!/bin/bash
# batch11.sh の続き (作業用のプロセスの再起動で neos859080 の回が止まったので回し直す)。結果は batch11.txt に足す
cd "$(dirname "$0")"
for s in 1 2 3 4 5 6 7 8; do
  ./run1.sh neos859080 neos859080_par3kx_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3kx ENOMOTO_MIP_OPTFACE_RACE_LOG=1 >> batch11.txt
done
echo ALLDONE >> batch11.txt
