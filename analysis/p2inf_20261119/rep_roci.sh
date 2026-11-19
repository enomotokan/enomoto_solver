#!/bin/sh
# P2-INF: MROWTINY で 2 種とも遅かった rocI-4-11 を 4 スレッド 600 秒 × 種 1〜4 × {def, rowtiny} でログつきで回す (logs/rocI-4-11_<設定>_s<種>.log)。
# リポジトリの根から: PYTHONPATH=<試作のビルド> sh analysis/p2inf_20261119/rep_roci.sh
D=analysis/p2inf_20261119
f=$(python3 -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('rocI-4-11',Path('.miplib_cache')))")
for s in 1 2 3 4; do for c in def rowtiny; do
  grep -q "^{" $D/logs/rocI-4-11_${c}_s$s.log 2>/dev/null && continue  # 終わった回は飛ばす (再開できる)
  E=""; [ $c = rowtiny ] && E=ENOMOTO_MIP_LPINF_ROWTINY=1
  env $E ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_LOG=1 python3 scripts/run_miplib_benchmark.py --worker-ours $f --time-limit 600 --threads 4 > $D/logs/rocI-4-11_${c}_s$s.log 2>&1
done; done
