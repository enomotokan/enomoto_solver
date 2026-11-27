#!/bin/bash
# 当方を 1 回解いてログを残す (heuradapt_20261124/one.sh の写し)。使い方 (どこからでも): bash analysis/fpstage3ls_20261125/one.sh <問題> <種> <秒> <スレッド> <札> [K=V ...]
# ログ: analysis/fpstage3ls_20261125/logs/<問題>_<札>_s<種>_t<スレッド>.log、結果の行は res_one.jsonl
cd "$(dirname "$0")/../.."
D=analysis/fpstage3ls_20261125; p=$1; s=$2; tl=$3; th=$4; tag=$5; shift 5
[ -f .miplib_cache/$p.mps ] || [ -f .miplib_cache/$p.mps.gz ] || .venv/bin/python -c "import sys; sys.path.insert(0,'scripts'); import run_miplib_benchmark as rb; from pathlib import Path; rb.fetch('$p', Path('.miplib_cache'))" > /dev/null
mps=$(.venv/bin/python -c "import sys; sys.path.insert(0,'scripts'); import run_miplib_benchmark as rb; from pathlib import Path; print(rb.fetch('$p', Path('.miplib_cache')))")
f=$D/logs/${p}_${tag}_s${s}_t${th}.log
env "$@" ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours $mps --time-limit $tl --threads $th > $f 2>&1
out=$(grep '^{' $f | tail -1)
echo "{\"problem\": \"$p\", \"tag\": \"$tag\", \"seed\": $s, \"threads\": $th, \"env\": \"$*\", \"result\": ${out:-null}}" >> $D/res_one.jsonl
echo "$p $tag s$s t$th $(echo $out | cut -c1-160)"
