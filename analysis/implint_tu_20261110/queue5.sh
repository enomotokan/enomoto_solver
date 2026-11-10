#!/bin/bash
# queue4.sh の後: v (変数上下限の行に出る列はカットの印から外す) を a と組み合わせた bav (wheel は tu2v3、既定・tu2a と同じ)
cd "$(dirname "$0")"
until grep -q QUEUE4_DONE /tmp/queue4.log; do sleep 30; done
export IMPLINT_PKG=/home/user/pkgs/tu2v3
python3 run.py b47.jsonl tu2av 1,2 600 mushroom-best neos-2978193-inde ns1830653
python3 run.py b58x.jsonl tu2av 1,2 60 @aff58
echo QUEUE5_DONE
