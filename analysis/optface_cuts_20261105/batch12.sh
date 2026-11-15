#!/bin/bash
# 利用者の案: par3ky (負けた写しのカットのうち勝った写しの LP のカットとの平行度が 0.5 以下のものを勝った写しの LP に直接足す) と
# par3kz (同じ条件のカットを LP には入れず主のカットプールにだけ入れて木のノードの伝播に使う)。beasleyC3・neos859080 を種 1〜8・300 秒。結果は batch12.txt
cd "$(dirname "$0")"
for prob in beasleyC3 neos859080; do
  for s in 1 2 3 4 5 6 7 8; do
    for c in ky kz; do
      ./run1.sh $prob ${prob}_par3${c}_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3$c ENOMOTO_MIP_OPTFACE_RACE_LOG=1 >> batch12.txt
    done
  done
done
echo ALLDONE >> batch12.txt
