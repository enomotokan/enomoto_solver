#!/bin/bash
# 使い方: S=作業場所 bt.sh <commit> [環境変数=値 ...]  -> $S/py/<commit> にビルド・展開し、fuzz_tiny_all 400 0 の食い違いを $S/out/<commit>$TAG.seeds に
S=${S:-/tmp/fz}
R=/home/user/enomoto_solver
c=$(git -C $R rev-parse --short=8 $1)
if [ ! -d $S/py/$c ]; then
  W=$S/wt/$c; [ -d $W ] || git -C $R worktree add -q --detach $W $c
  rm -rf $S/wh/$c; (cd $W && CARGO_TARGET_DIR=$S/target maturin build --release -q -o $S/wh/$c >$S/wh_$c.log 2>&1) || { echo "BUILD FAIL $c"; exit 1; }
  mkdir -p $S/py/$c && (cd $S/py/$c && unzip -q $S/wh/$c/*.whl)
fi
mkdir -p $S/out
cd $R && env "${@:2}" PYTHONPATH=$S/py/$c python $(dirname $(readlink -f $0))/fuzz_tiny_all.py 400 0 2>&1 | grep -E "^(seed|bad|Trace|[A-Za-z]*Error)" > $S/out/$c${TAG}.seeds
echo "$c $(tail -1 $S/out/$c${TAG}.seeds)"
