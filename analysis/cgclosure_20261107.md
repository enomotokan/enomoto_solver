# CG カットを全部使えたら根のギャップがどれだけ閉じるか: ランク 1 の CG 閉包・mod-k 閉包・mod-2 閉包の近似 (2026-11-07)

bench に使う環境変数: ENOMOTO の根は `ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_NODE_LIMIT=1` (既定の設定でノード 1 つで止める)。
HiGHS の根は `ENOMOTO_BENCH_HIGHS_OPT=mip_max_nodes=1` (4 スレッド)。どちらも時間の上限 120 秒。閉包はソルバーを使わず、highspy 1.15.1 で
分離 MIP を解く Python のスクリプト (`cgclosure_20261107/cgclosure.py`)。ENOMOTO のソースは変えていない (診断の環境変数も足していない)。

(計測中。結論はまとまってから書く。)
