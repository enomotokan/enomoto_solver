#!/bin/bash
# build.sh <sha>  -> /home/user/wt/regress1-builds/<short>/.pylib
set -e
sha=$(git -C /home/user/enomoto_solver rev-parse --short=7 $1)
B=/home/user/wt/regress1-builds/$sha
if [ -f $B/.pylib/enomoto_solver/__init__.py ]; then echo "have $sha"; exit 0; fi
cd /home/user/wt/regress1-old
git checkout -q --detach $sha
export CARGO_TARGET_DIR=/home/user/wt/regress1-old/target CARGO_BUILD_JOBS=4
mkdir -p $B; rm -f $B/*.whl
/tmp/with_slot.sh maturin build --release -o $B > $B/build.log 2>&1
pip install -q --no-deps --force-reinstall --target $B/.pylib $B/*.whl > /dev/null 2>&1
echo "built $sha"
