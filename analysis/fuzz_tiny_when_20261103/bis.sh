#!/bin/bash
# bis.sh <good> <bad> <seed>: first-parent の列で二分探索。ビルドできないコミットは隣へ
S=${S:-/tmp/fz}; R=/home/user/enomoto_solver
seed=$3
mapfile -t L < <(git -C $R rev-list --first-parent --reverse --abbrev-commit --abbrev=8 $1..$2)
lo=-1; hi=$((${#L[@]}-1))   # L[hi] は bad、lo は good (-1 = $1)
while [ $((hi-lo)) -gt 1 ]; do
  m=$(((lo+hi)/2)); c=${L[$m]}
  if ! $(dirname $(readlink -f $0))/bt.sh $c >/dev/null 2>&1 || ! grep -q "^bad" $S/out/$c.seeds; then
    echo "skip $c (build/run fail)"; L[$m]=${L[$((m+1))]}; 
    # 隣と入れ替え: 単純に m を bad 側に寄せて m+1 を試す
    c=${L[$m]}; $(dirname $(readlink -f $0))/bt.sh $c >/dev/null 2>&1 || { echo "fail again $c"; exit 1; }
  fi
  if grep -q "^seed $seed " $S/out/$c.seeds; then hi=$m; r=bad; else lo=$m; r=good; fi
  echo "$c $r $(tail -1 $S/out/$c.seeds)"
done
echo "FIRST BAD for seed $seed: ${L[$hi]}"
