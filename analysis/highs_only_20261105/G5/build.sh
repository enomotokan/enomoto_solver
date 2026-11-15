#!/bin/bash
# 計測の合間にビルドする: 錠を置き、走っている求解が終わるのを待ってからビルドし、錠を外す
touch /tmp/G5_build.lock
while pgrep -f "worker-(ours|highs)" > /dev/null; do sleep 5; done
cd /home/user/enomoto_solver && rm -f /tmp/whl/*.whl && maturin build --release -o /tmp/whl > /tmp/build2.log 2>&1 && pip install -q --force-reinstall --no-deps /tmp/whl/*.whl >> /tmp/build2.log 2>&1
echo "BUILD rc=$? $(date)" >> /tmp/build2.log
rm -f /tmp/G5_build.lock
