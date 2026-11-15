#!/bin/bash
# 並びの組み直し (queue.sh・queue2.sh の続き): b58x の残り → 診断 F → 47 問の一部 (600 秒)。どれも途中から再開できる
cd "$(dirname "$0")"
while pgrep -f "run.py b58x" >/dev/null; do sleep 20; done
export IMPLINT_PKG=/home/user/pkgs/tu2v3
python3 run.py b58x.jsonl tu2 1,2 60 @aff58
python3 run.py b58x.jsonl tu2as 1,2 60 exp-1-500-5-5 pk1 gr4x6 khb05250 assign1-5-8
echo B58X_DONE
# 診断 F 用の wheel (F の文字は tu2v3 に無い)
WT=$(cd ../.. && pwd)
if [ ! -d /home/user/pkgs/tu2v4 ]; then
  (cd "$WT" && maturin build --release -o /tmp/whlF > /tmp/buildF.log 2>&1)
  pip install -q --no-deps --target /home/user/pkgs/tu2v4 $(ls -t /tmp/whlF/*.whl | head -1)
fi
IMPLINT_PKG=/home/user/pkgs/tu2v4 python3 run.py bF.jsonl def,tu2aF 1,2 60 mc11 sp150x300d binkar10_1
echo BF_DONE
python3 run.py b47.jsonl def,tu2a 1,2 600 @aff47
python3 run.py b47.jsonl tu2as 1,2 600 csched008 tr12-30
echo QUEUE_DONE
