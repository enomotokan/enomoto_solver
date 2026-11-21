#!/bin/sh
# glc の誤答の再現の 1 回: one.sh TAG スレッド 種 [KEY=VAL ...] (問題は環境変数 PROB、既定 neos-4722843-widden、600 秒)。
# 結果は TAG.json (worker の出力) と TAG.log (stderr)。ENOMOTO_MIP_GEN_OLD・LIFTED_COVER は引数で付けたときだけ効く
R=$(cd "$(dirname "$0")/../../.." && pwd)
PROB=${PROB:-neos-4722843-widden}
TAG=$1; TH=$2; SEED=$3; shift 3
env -u ENOMOTO_MIP_GEN_OLD -u ENOMOTO_MIP_LIFTED_COVER "$@" ENOMOTO_MIP_THREADS=$TH ENOMOTO_T_MIP_SEED=$SEED \
  python $R/scripts/run_miplib_benchmark.py --worker-ours $R/.miplib_cache/$PROB.mps --time-limit 600 --threads $TH > $TAG.json 2> $TAG.log
python -c "
import json
l=[x for x in open('$TAG.json') if x.startswith('{')][-1]; r=json.loads(l)
print('$TAG', r['status'], r.get('objective'), r.get('bound'), round(r['time'],1), r.get('nodes'))"
