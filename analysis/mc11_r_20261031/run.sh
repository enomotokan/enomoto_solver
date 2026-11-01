#!/bin/bash
# 1 問を 1 回解く。使い方: run.sh 問題 名前 設定 [追加の環境変数 ...]
# 設定: def / bgw (hs_pi0_bgw) / r (hs_pi0_bgw_r)。ログは logs/名前.log (標準エラー)、結果の JSON は logs/名前.json
cd "$(dirname "$0")/../.."
prob=$1; name=$2; cfg=$3; shift 3
d=analysis/mc11_r_20261031/logs
HS="ENOMOTO_MIP_SUBMIP_HIGHS=-s ENOMOTO_MIP_SUBMIP_PC_INHERIT=1 ENOMOTO_T_MIP_SUBMIP_RELIABLE=0 ENOMOTO_MIP_BG_CUTS_WAIT=1"
case $cfg in
  def) E="" ;;
  bgw) E="$HS" ;;
  r) E="$HS ENOMOTO_MIP_SUBMIP_HIGHS_TRIM=r" ;;
  *) echo "unknown cfg $cfg"; exit 1 ;;
esac
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 $E "$@" python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$prob.mps \
  --time-limit 60 --threads 4 2> $d/$name.log | tail -1 > $d/$name.json
echo "$name $(cat $d/$name.json)"
