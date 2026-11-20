#!/bin/bash
# a9d7bae3d 系 (旧方式: 際どい実行不能は論理変数基底から解き直し → 結論なし → LP 値なしの分枝) で lpinf が大量に出る問題を探す。
# 旧ビルド (ec54d0cb0) を /tmp/venv_old に入れて、M の問題を 60 秒・種 1 で既定と NO_LPINF_CHECK で回す
D=$(cd "$(dirname "$0")" && pwd)
PY=${PY:-/tmp/venv_old/bin/python}
for p in rococoC10-001000 timtab1 mas74 mas76 neos-3627168-kasai fastxgemm-n2r6s0t2 ran14x18-disj-8 CMS750_4 istanbul-no-cutoff tr12-30 supportcase40 neos-662469 roll3000 sct2; do
  nice -n 5 $PY "$D/one.py" $p old_def 1 60
  nice -n 5 $PY "$D/one.py" $p old_off 1 60 ENOMOTO_MIP_NO_LPINF_CHECK=1
done
