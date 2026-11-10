#!/bin/bash
# queue.sh の後: 診断 F (一般のネットワーク行列の判定を入れたら効くかの見積もり) 用の wheel を作って計測する
cd "$(dirname "$0")"
until grep -q QUEUE_DONE /tmp/queue.log; do sleep 30; done
WT=$(cd ../.. && pwd)
(cd "$WT" && maturin build --release -o /tmp/whlF > /tmp/buildF.log 2>&1)
rm -rf /home/user/pkgs/tu2v4 && pip install -q --no-deps --target /home/user/pkgs/tu2v4 $(ls -t /tmp/whlF/*.whl | head -1)
export IMPLINT_PKG=/home/user/pkgs/tu2v4
python3 run.py bF.jsonl def,tu2aF 1,2 60 mc11 sp150x300d binkar10_1
echo QUEUE2_DONE
