#!/bin/bash
# wheel を作って入れ直す
cd /home/user/enomoto_solver && rm -f dist/*.whl && maturin build --release -o dist 2>&1 | grep -E "^error|Built wheel" ; pip install -q --no-deps --force-reinstall dist/*.whl 2>&1 | grep -v WARNING
