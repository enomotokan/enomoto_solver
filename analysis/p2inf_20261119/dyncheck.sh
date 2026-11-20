#!/bin/bash
# P2-INF: 途中で衝突の重みを上げる試作 (CONFLICT_WEIGHT_DYN) の確かめ。問題:種:秒:設定,... を 4 スレッドで順に回し logs/dyncheck.txt に足す。
# 環境変数 TH (スレッド数、既定 4)・OUT (書き足す表の名前、既定 dyncheck)。
# 設定: def / dyn (DYN=0.1) / dynlx (DYN=0.1 + LIT_EXPLAIN) / ramp (RAMP=0.1、シグモイドの段階形) / ramplx (RAMP=0.1 + LIT_EXPLAIN) / rampax = cwr (RAMP=0.1 + LIT_EXPLAIN=auto、上げた間だけ) / cwrha (cwr + RAMP_HCS、付け方は最初から HiGHS 式、旧 cwrh) / cwrhaf (cwrha + σ < 0.05 で重み 0) / cwrh (RAMP 0.1 + RAMP_HCS + 床 0.05 + LIT_EXPLAIN=sig: 遡る数を解析ごとに round(16σ) まで) / cwrh0 (cwrh から床を外す) / cwm (cwrh0 + 部分木ごとの倍率を対で比べて寄せる、ENOMOTO_MIP_CWM=1) / cwmfix (cwm の学習なし: m=0 対 m=1 をいつも比べる)。リポジトリの根から: PYTHONPATH=<ビルド> bash analysis/p2inf_20261119/dyncheck.sh 問題:種:秒:設定,設定 ...
cd "$(dirname "$0")/../.."
O=analysis/p2inf_20261119/logs/${OUT:-dyncheck}.txt; TH=${TH:-4}; T=; [ $TH = 4 ] || T=_t$TH
for spec in "$@"; do IFS=: read p s tl cs <<< "$spec"
  f=$(python3 -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('.miplib_cache')))")
  for c in ${cs//,/ }; do
    case $c in def) E="X=1";; dyn) E="ENOMOTO_MIP_CONFLICT_WEIGHT_DYN=0.1";; dynlx) E="ENOMOTO_MIP_CONFLICT_WEIGHT_DYN=0.1 ENOMOTO_MIP_LIT_EXPLAIN=1";; ramp) E="ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1";; ramplx) E="ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1 ENOMOTO_MIP_LIT_EXPLAIN=1";; rampax|cwr) E="ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1 ENOMOTO_MIP_LIT_EXPLAIN=auto";; cwrha) E="ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1 ENOMOTO_MIP_LIT_EXPLAIN=auto ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP_HCS=1";;  cwrhaf) E="ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1 ENOMOTO_MIP_LIT_EXPLAIN=auto ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP_HCS=1 ENOMOTO_T_MIP_CW_RAMP_FLOOR=0.05";; cwrh) E="ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1 ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP_HCS=1 ENOMOTO_T_MIP_CW_RAMP_FLOOR=0.05 ENOMOTO_MIP_LIT_EXPLAIN=sig";; cwrh0) E="ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1 ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP_HCS=1 ENOMOTO_MIP_LIT_EXPLAIN=sig";; cwm) E="ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1 ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP_HCS=1 ENOMOTO_MIP_LIT_EXPLAIN=sig ENOMOTO_MIP_CWM=1";; cwmfix) E="ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1 ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP_HCS=1 ENOMOTO_MIP_LIT_EXPLAIN=sig ENOMOTO_MIP_CWM=fix";; esac
    log=analysis/p2inf_20261119/logs/dyn_${p}_${c}_s$s$T.log
    env $E ENOMOTO_MIP_THREADS=$TH ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_LOG=1 python3 scripts/run_miplib_benchmark.py --worker-ours $f --time-limit $tl --threads $TH > $log 2>&1
    r=$(grep '^{' $log | python3 -c 'import sys,json;r=json.loads(sys.stdin.read());print(r["status"],round(r["time"],1),r["nodes"])')
    raised=$(grep -c "CW_DYN\|CW_RAMP" $log)
    dg=$(grep "conflict score diag" $log | tail -1 | sed 's/.*analyses \([0-9]*\), bumped columns per analysis \([0-9.]*\), analyses without conflict \([0-9.]*\)/an=\1 bump=\2 nocf=\3/')
    lb=$(grep "lit explain budget" $log | tail -1 | sed 's/.*resolved \([0-9]*\), analyses cut by the budget \([0-9]*\).*/lxres=\1 lxcut=\2/')
    cw=$(grep "CWM final" $log | tail -1 | sed 's/.*CWM final: \(.*best m=[0-9.]*\).*/\1/')
    echo "$p s$s$T $c $r raised=$raised $dg $lb ${cw:+cwm: $cw}" | tee -a $O
  done
done
