#!/bin/sh
# 使い方: one.sh 名前 問題 秒 [環境変数=値 ...]  (既定 + 診断 DENSECUT_DIAG、4 スレッド)
cd /home/user/enomoto_solver
O=$(cd "$(dirname "$0")" && pwd)/logs
n=$1; p=$2; tl=$3; shift 3
m=$(python -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('.miplib_cache')))")
env "$@" ENOMOTO_MIP_DENSECUT_DIAG=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 python scripts/run_miplib_benchmark.py --worker-ours $m --time-limit $tl --threads 4 > $O/$p.$n.out 2> $O/$p.$n.log
python3 -c "
import json,sys
r=json.loads([l for l in open('$O/$p.$n.out') if l.startswith('{')][-1])
print('$p','$n',r.get('status'),round(r.get('time',0),1),r.get('objective'),r.get('bound'),r.get('nodes'))"
grep -h "DENSECUT\|S3_TSDENSE" $O/$p.$n.log | head -5 | cut -c1-300
