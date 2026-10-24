# 計測の依頼 (62ef950、サブ MIP を HiGHS に寄せる SUBMIP_HIGHS=-t、2026-10-24)

あなたは enomoto_solver (Rust 製の MIP ソルバー、GitHub の enomotokan/enomoto_solver) の計測セッション (組 K) です。報告・コミットメッセージは日本語で書いてください。調査や実装はしない。計測だけをする。

1. ブランチ inv/submip-20261024 の 62ef950 を使う (既定は mixoc + fix9pe。サブ MIP を HiGHS に寄せる切り替え ENOMOTO_MIP_SUBMIP_HIGHS を足したもの。既定は不変)。作業ブランチ inv/submip11-bench-20261024-s<K> を 62ef950 から作り、ここにだけ push する (develop・inv/heur-ls-20261023・inv/submip-20261024・claude/* には push しない)。
2. ビルド: pip install highspy==1.15.1 maturin scipy、maturin build --release -o dist、pip install --no-deps --force-reinstall dist/*.whl。MPS は git fetch origin inv/mps-cache-20261021 && mkdir -p .miplib_cache && for f in $(git ls-tree --name-only origin/inv/mps-cache-20261021 mps/); do git show origin/inv/mps-cache-20261021:$f | gunzip > .miplib_cache/$(basename $f .gz); done。nproc が 4 未満なら計測せずに親に知らせて止まる。
3. 担当の問題 (ファイル bench_subsets.txt は使わない): 下の表の組 K の行。P="その行の問題名をスペース区切り"
4. 計測: 次の 4 回を、この順に 1 本ずつ (同時に 1 つ、計測中にビルドしない)、Bash の run_in_background (timeout 7200000) で回す (4 回を順に回す台本 1 本にしてよい)。sleep を連ねない。setsid・nohup で切り離さない。各回は
   ENOMOTO_BENCH_ONLY="$P" ENOMOTO_BENCH_SHA=62ef950 analysis/heur_ls_20261023/bench_mix.sh <名前>_s<K> <組> [ENV=VAL ...]
   組 HLs: (1) def11 (何も付けない) (2) smh11 ENOMOTO_MIP_SUBMIP_HIGHS=-t
   組 HLt: (3) smh11 ENOMOTO_MIP_SUBMIP_HIGHS=-t (4) def11 (何も付けない)
   途中で落ちたら、同じコマンドを回し直せば続きから再開する。
5. 終わったら benchmarks/ の *_62ef950_* の json・md と benchmarks/cand_logs/ を commit して inv/submip11-bench-20261024-s<K> に push する。
6. 担当の問題ごとに、4 回の (状態・目的値・時間) を短く、目的値の不一致 (状態が optimal なのに json の highs の目的値と違う) の有無とあわせて、日本語 5 行以内で親に send_message (to: @parent) で送り、同じ内容を最後の返答にする。
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
