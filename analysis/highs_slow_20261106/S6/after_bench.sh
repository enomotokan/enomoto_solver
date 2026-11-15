#!/bin/bash
# 58 問の後: nexp の p2r (TCAP + 第 2 段階のラウンドの上限 200) を 3 回、続けて cargo test --release --lib
cd "$(dirname "$0")"
while pgrep -f "\./after_queue\.sh" >/dev/null; do sleep 20; done
./queue.sh > queue2.out 2>&1
cd ../../..
RUSTFLAGS="-C link-arg=-L/usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib cargo test --release --lib > /tmp/cargo_test_s6.log 2>&1
echo "exit $?" >> /tmp/cargo_test_s6.log
