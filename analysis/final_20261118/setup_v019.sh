#!/bin/sh
# PyPI の 0.1.9 を別の venv (<repo>/.v019env) に入れる (run.py の v019 が使う)。今の版は通常どおり pip install -e . で入れる。
set -e
R=$(cd "$(dirname "$0")/../.." && pwd)
python3 -m venv "$R/.v019env"
"$R/.v019env/bin/pip" install -q enomoto-solver==0.1.9 highspy==1.15.1
"$R/.v019env/bin/python" -c "import enomoto_solver, importlib.metadata as m; print('v019', m.version('enomoto-solver'), enomoto_solver.__file__)"
