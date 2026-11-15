#!/bin/bash
# queue.sh の後: (1) 差が出た問題の 2 回目 (base・fix)、(2) 幅の閾値を 1e3 に下げた形 (fixr1e3) で新たに当たる問題を base と比べる
cd "$(dirname "$0")"
while pgrep -f "queue.sh" >/dev/null && pgrep -f "run.py .* b300" >/dev/null && ! grep -q QUEUE_DONE /tmp/queue.log; do sleep 30; done
run() { grep -q "\"problem\": \"$2\", \"cfg\": \"$1\", \"rep\": $3," $4.jsonl 2>/dev/null || python3 run.py $1 $2 300 $3 $4; }
for p in map10 map16715-04 neos-3656078-kumeu satellites2-60-fs neos-2987310-joes neos-4763324-toguru irish-electricity neos-4722843-widden neos-5188808-nattai app1-2; do
  for c in base fix; do run $c $p 1 b300; done
done
for p in atlanta-ip sing326 snp-04-052-052 k1mushroom piperout-d20 neos-1367061; do
  for c in base fixr1e3; do run $c $p 0 thr; done
done
echo QUEUE2_DONE
