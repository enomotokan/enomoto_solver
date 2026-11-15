#!/bin/bash
# 束 cand1114 の環境変数を付けてコマンドを回す: ./envc.sh コマンド...
eval "$(python3 -c "
import sys; sys.path.insert(0, '$(dirname $0)/../cand1114_20261113')
from cfg import CAND1114
print(' '.join(f'export {k}={v};' for k, v in CAND1114.items()))
")"
exec "$@"
