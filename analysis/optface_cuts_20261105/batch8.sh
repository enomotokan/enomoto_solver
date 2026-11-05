#!/bin/bash
# 利用者の案 (最初の数ラウンドだけ独立に分離し、全部のカットをまとめて選別し、そこから既定の並列の分離): par3m (ENOMOTO_MIP_OPTFACE_PARCUTS=3m)。
# 今のビルドで beasleyC3・neos859080 を種 1〜8・300 秒、neos859080 の既定も今のビルドで回し直す (defnew)。結果は batch8.txt
cd "$(dirname "$0")"
for s in 1 2 3 4 5 6 7 8; do
  ./run1.sh beasleyC3 beasleyC3_par3m_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3m >> batch8.txt
done
for s in 1 2 3 4 5 6 7 8; do
  ./run1.sh neos859080 neos859080_par3m_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3m >> batch8.txt
  ./run1.sh neos859080 neos859080_defnew_s$s 4 $s 300 >> batch8.txt
done
echo ALLDONE >> batch8.txt
