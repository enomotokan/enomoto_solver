#!/bin/sh
# 1 問を診断つき (ENOMOTO_MIP_POOL3_DIAG・ENOMOTO_MIP_LBNODE_DIAG・ENOMOTO_MIP_LOG) で解く (run_one.sh の設定を増やしたもの)。
# 使い方: sh run2.sh <問題> <設定> <秒> <スレッド数>
# 設定: def (既定) / p3_5・p3_05・p3_005 (ENOMOTO_MIP_POOL3_BEST=5・0.5・0.05) / tk (ENOMOTO_MIP_POOL3_TAKEN=1) / tkp3 (TAKEN=1 と BEST=0.5)
# 結果は <問題>_<設定>_<秒>s_t<スレッド>.out (worker の JSON) と .err.gz (ログ)。
D=$(dirname "$0"); W=$D/../..
P=$1; C=$2; T=$3; N=$4; NAME=${P}_${C}_${T}s_t${N}
case $C in
  def) EXTRA="" ;;
  p3_5) EXTRA="ENOMOTO_MIP_POOL3_BEST=5" ;;
  p3_05) EXTRA="ENOMOTO_MIP_POOL3_BEST=0.5" ;;
  p3_005) EXTRA="ENOMOTO_MIP_POOL3_BEST=0.05" ;;
  tk) EXTRA="ENOMOTO_MIP_POOL3_TAKEN=1" ;;
  tkp3) EXTRA="ENOMOTO_MIP_POOL3_TAKEN=1 ENOMOTO_MIP_POOL3_BEST=0.5" ;;
  *) echo "unknown cfg $C"; exit 1 ;;
esac
env ENOMOTO_MIP_POOL3_DIAG=1 ENOMOTO_MIP_LBNODE_DIAG=1 ENOMOTO_MIP_LOG=1 $EXTRA \
  python "$W/scripts/run_miplib_benchmark.py" --worker-ours "$W/.miplib_cache/$P.mps" --time-limit "$T" --threads "$N" \
  > "$D/$NAME.out" 2> "/tmp/claude-0/$NAME.err"
gzip -c "/tmp/claude-0/$NAME.err" > "$D/$NAME.err.gz"
