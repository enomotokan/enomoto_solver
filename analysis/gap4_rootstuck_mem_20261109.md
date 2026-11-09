# GAP4: 根から出られない問題とメモリの元 (2026-11-09)

bench に使う環境変数: `ENOMOTO_MIP_GAP4=<文字>` (試作、既定では使わない。`i`・`p`・`r`・`o`・`c` の組み合わせ。58 問は `iproc`)、
診断 `ENOMOTO_MIP_GAP4_RSS=<秒>` (RSS と生きている `Solver` の数を秒おきに出す。動きは変えない)、既存の `ENOMOTO_MIP_MEMB_DIAG=1`
(開いたノードの持ち物)、既存の `ENOMOTO_MIP_TREE_NO_PASS_BASIS=1` (共有の待ち行列のノードに基底・DSE 重みを付けない)。
計測は `ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=s`、4 スレッド、ソルバーは同時に 1 つ。HiGHS は highspy 1.15.1
(threads=4・mip_rel_gap=0・output_flag 真)。

(作業中。下の節を順に埋める)
