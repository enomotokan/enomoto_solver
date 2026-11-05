#!/bin/bash
# batch2 の後: (1) beasleyC3 の par を直したビルド (確かめの解き直しの期限の誤りを直した 13:23:41 の後) で種 1〜8 回し直す (名前 par2)。
# (2) ビルド中 (13:20:35〜13:23:40) に走った beasleyC3 の種 6 を全設定で回し直す (名前 s6r)。結果の 1 行は batch3.txt
cd "$(dirname "$0")"
until grep -q ALLDONE batch2.txt 2>/dev/null; do sleep 10; done
for s in 1 2 3 4 5 6 7 8; do
  ./run1.sh beasleyC3 beasleyC3_par2_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3 >> batch3.txt
done
declare -A CFG=( [def]="" [rob]="ENOMOTO_MIP_ROBUST_ROOTIPM=1" [both]="ENOMOTO_MIP_ROOTLP_BOTH=1" [mv]="ENOMOTO_MIP_OPTFACE_MOVE=f" [par2]="ENOMOTO_MIP_OPTFACE_PARCUTS=3" )
for c in def rob both mv; do
  ./run1.sh beasleyC3 beasleyC3_${c}_s6r 4 6 300 ${CFG[$c]} >> batch3.txt
done
echo ALLDONE >> batch3.txt
