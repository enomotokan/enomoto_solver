# 計測の依頼 (d20cf31、mixoc を既定にした後の候補の組、2026-10-24)

あなたは enomoto_solver (Rust 製の MIP ソルバー、GitHub の enomotokan/enomoto_solver) の計測セッション (組 K) です。報告・コミットメッセージは日本語で書いてください。調査や実装はしない。計測だけをする。

1. ブランチ claude/enomoto-solver-heuristic-survey-xjlnol の d20cf31 を使う (mixoc を既定にし、各調査の切り替えを merge したもの)。作業ブランチ inv/combo9-bench-20261024-s<K> を d20cf31 から作り、ここにだけ push する (develop・inv/heur-ls-20261023・inv/combo-20261023・inv/mc11bisect-20261023・claude/* には push しない)。
2. ビルド: pip install highspy==1.15.1 maturin scipy、maturin build --release -o dist、pip install --no-deps --force-reinstall dist/*.whl。MPS は git fetch origin inv/mps-cache-20261021 && mkdir -p .miplib_cache && for f in $(git ls-tree --name-only origin/inv/mps-cache-20261021 mps/); do git show origin/inv/mps-cache-20261021:$f | gunzip > .miplib_cache/$(basename $f .gz); done。nproc が 4 未満なら計測せずに親に知らせて止まる。
3. 担当の問題は下の表の組 K の行 (ファイル bench_subsets.txt は使わない)。P="その行の問題名をスペース区切り"
4. 環境変数の組 (どれも文字列そのまま。既定は mixoc に変わっているので、何も付けない回が新しい既定):
   CAND="ENOMOTO_MIP_WORKER_HALT=1 ENOMOTO_MIP_LATE_NO_RESOLVE=1 ENOMOTO_MIP_LP_PROBE_FINISH=1 ENOMOTO_MIP_LP_PROBE_ROW_BASIS=1 ENOMOTO_MIP_POSTSOLVE_REPAIR=1 ENOMOTO_MIP_GR_RESTART=1 ENOMOTO_MIP_FJ_NOSOL_EXTEND=2 ENOMOTO_MIP_CUT_STRONG_WIN5=nofc ENOMOTO_MIP_CUT_INTOBJ_STALL=1 ENOMOTO_MIP_HEUR_MU_ROLE_OBS=1 ENOMOTO_MIP_SYM_SKIP_FIXED=1 ENOMOTO_MIP_CUT_AGG=1 ENOMOTO_MIP_RESTART_BASIS=1"
   CANDSB="$CAND ENOMOTO_MIP_SB_BUDGET_HIGHS=1"
5. 計測: 次の 6 回を、この順に 1 本ずつ (同時に 1 つ、計測中にビルドしない)、Bash の run_in_background (timeout 7200000) で回す (6 回を順に回す台本 1 本にしてよい)。sleep を連ねない。setsid・nohup で切り離さない。各回は
   ENOMOTO_BENCH_ONLY="$P" ENOMOTO_BENCH_SHA=d20cf31 analysis/heur_ls_20261023/bench_mix.sh <名前>_s<K> <組> [ENV=VAL ...]
   組 HLq: (1) def9 (何も付けない) (2) cand9 $CAND (3) candsb9 $CANDSB
   組 HLr: (4) candsb9 $CANDSB (5) cand9 $CAND (6) def9 (何も付けない)
   途中で落ちたら、同じコマンドを回し直せば続きから再開する。
6. 終わったら benchmarks/ の *_d20cf31_* の json・md と benchmarks/cand_logs/ を commit して inv/combo9-bench-20261024-s<K> に push する。
7. 担当の問題ごとに、6 回の (状態・目的値・時間) を短く、目的値の不一致 (状態が optimal なのに json の highs の目的値と違う) の有無とあわせて、日本語 5 行以内で親に send_message (to: @parent) で送り、同じ内容を最後の返答にする。
目的値の不一致が出たら、その場で親に最優先で知らせてください。


| 組 K | 問題 |
|---|---|
| 1 | gen-ip002 p0201 10teams binkar10_1 h80x6320d nursesched-sprint02 physiciansched6-2 |
| 2 | gen-ip054 misc07 air03 neos-860300 supportcase26 hypothyroid-k1 uccase12 |
| 3 | pk1 flugpl blend2 eil33-2 timtab1 ns1116954 neos-2978193-inde |
| 4 | gr4x6 dcmulti fiber nw04 bppc4-08 neos-950242 mzzv42z |
| 5 | markshare_4_0 gt2 noswot assign1-5-8 cvs16r128-89 drayage-25-23 neos-960392 |
| 6 | mas74 khb05250 qiu beavma mik-250-20-75-4 drayage-100-23 roll3000 |
| 7 | mas76 qnet1 neos-911970 blp-ar98 neos-1456979 beasleyC3 app1-1 |
| 8 | neos5 rout 30n20b8 ran14x18-disj-8 graph20-20-1rand mc11 |
