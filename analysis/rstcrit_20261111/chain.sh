#!/bin/sh
# 手元の順: cargo test の終わりを待って分担 3 (木の再スタートの ti・tt は TREEEFF に任せたので回さない)
cd "$(dirname "$0")"
while pgrep -f "cargo test" >/dev/null; do sleep 20; done
python run_share.py 3 --live >> share3.log 2>&1
cd .. && cd .. && RUSTFLAGS="-L /usr/local/lib -l python3.13" cargo test --release > analysis/rstcrit_20261111/cargo_test.log 2>&1
cd analysis/rstcrit_20261111 && for p in neos-911970 uccase12 neos-4722843-widden tbfp-network; do for s in 1 2; do for c in def f25 n05; do echo "$c $p 600 $s resM.live.jsonl"; done; done; done > jobsM.txt && python queue.py jobsM.txt > queueM.log 2>&1
