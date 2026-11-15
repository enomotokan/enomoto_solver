#!/bin/bash
# 取得済みの候補を順に scan.py に流す (取得が終わるまで繰り返す)
cd "$(dirname "$0")"
while true; do
  ps=$(python3 -c "import json;print(' '.join(json.loads(l)['problem'] for l in open('meta_orig.jsonl')))")
  python3 scan.py 40 4 $ps
  if ! pgrep -f rootlp_scale_20261106/fetch.py >/dev/null; then
    ps2=$(python3 -c "import json;print(' '.join(json.loads(l)['problem'] for l in open('meta_orig.jsonl')))")
    [ "$ps" == "$ps2" ] && break
  fi
done
echo SCAN_DONE
