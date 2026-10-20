#!/bin/bash
# build.sh <sha|dir>  -> /home/user/wt/b/<short>/.pylib ; 引数がディレクトリならその作業木をビルド (名前は basename)
set -e
if [ -d "$1" ]; then W=$1; name=$(basename $1); else
  name=$(git -C /home/user/enomoto_solver rev-parse --short=7 $1); W=/home/user/wt/w/$name
  [ -d $W ] || git -C /home/user/enomoto_solver worktree add -f --detach $W $name >/dev/null 2>&1
fi
B=/home/user/wt/b/$name
mkdir -p $B; rm -rf $B/*.whl $B/.pylib
export CARGO_TARGET_DIR=/home/user/wt/target CARGO_BUILD_JOBS=4
(cd $W && maturin build --release -o $B > $B/build.log 2>&1)
pip install -q --no-deps --force-reinstall --target $B/.pylib $B/*.whl > /dev/null 2>&1
echo "built $name -> $B/.pylib"
