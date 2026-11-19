#!/bin/sh
# git bisect run 用 (worktree の中で呼ぶ): 種 1・2 とも 90 秒で実行不能なら良い (0)、どちらも時間切れなら悪い (1)、
# 割れたら種 3 で決める。ビルドに失敗したら飛ばす (125)。
D=/home/user/enomoto_solver/analysis/p2inf_20261119
SHA=$(git rev-parse --short=9 HEAD)
sh "$D/bis.sh" "$SHA" 90 1,2 >/dev/null 2>&1 || exit 125
n=$(grep "\"sha\":\"$SHA\"" "$D/res_bis.jsonl" | grep -c '"infeasible"')
t=$(grep -c "\"sha\":\"$SHA\"" "$D/res_bis.jsonl")
[ "$t" -ge 2 ] || exit 125
if [ "$n" -eq 1 ]; then
  sh "$D/bis.sh" "$SHA" 90 3 >/dev/null 2>&1 || exit 125
  n=$(grep "\"sha\":\"$SHA\"" "$D/res_bis.jsonl" | grep -c '"infeasible"')
fi
[ "$n" -ge 2 ] && exit 0 || exit 1
