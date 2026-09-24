# presolve パイプライン (単体法反復・LU 以外) の非効率: 原因特定と対処法 (2026-09-24)

担当範囲: `model.solve(root_solver=None)` の呼び出しから戻るまでのうち、単体法反復ループと LU 以外
(presolve 各パス、スケーリング、`build_a_g`、標準形構築、ソルバ起動時の初期化、postsolve、FFI)。
基点: `main` HEAD `8cc4303` (= `scratchpad/venv_base` のビルド)。**src/ は変更していない**。計測用の計装は
別 worktree (`scratchpad/ps_wt`、ブランチ `ps-instrument-20260924`、専用 venv `scratchpad/venv_ps`) に入れた。
生データ・スクリプト: `analysis/presolve_pipeline_20260924_data/` (`full_ps.json` 段階別時間、`base3.json`
パス別カウンタ、`rounds.json`、`var_*.json` 変種計測、`cg_*_incl.txt` callgrind、`sweep.py` 等)。

採用基準 (指示): 全 93 問の実行時間の幾何平均の改善、かつ各問題で 10% 超の退行なし。幾何平均では
afiro (0.13 ms) の 10% と dfl001 (8 s) の 10% が同じ重みなので、小問題の固定費 = presolve が主戦場になる。
本報告の「幾何平均効果」はすべて 93 問の比率の幾何平均。

注意: 途中で API レート制限により中断されたため、§4 の変種計測は予定 35 種のうち 8 種
(reduce_eq / reduce_ineq(round) / parallelcols / dualpropagate / doubleton / aggregator / eqprop の各無効化、
Ruiz 0 反復 [23 問のみ]) で打ち切り。残り (Ruiz 3/5/20、round 上限 3/5、propagate pass 数、相対許容、
密/疎閾値、DM 分解、rayon 無効、colsingleton/dualfix/foldfixed/freevar 無効、fixpoint 厳密、row-local
aggregator、fill-in 打ち切りなし、ineqsingleton) は計装ビルドの `ENOMOTO_PS_*` env で同じ手順
(`sweep.py --no-clean --prof-env ENOMOTO_PROF_PIPE=1 <ENV>` → `variant_table.py`) を回せば得られる。

## 0. 計測方法

1. **段階別壁時計 (全 93 問)**: HEAD のコードに `Instant::now()` 積算だけの計装 (`pipeprof`;
   `ENOMOTO_PROF_PIPE=1` で有効、1 solve につき最後に 1 行だけ eprintln) を入れたビルドで、問題ごとに新
   プロセス、モデル構築後 solve を 0.6 s 分 (3〜30 回) 繰り返し、各段階の**最小値**を採用 (4 コアで他
   エージェントも計測中のため中央値より最小値が安定; 負荷平均 5)。計装なし同一ビルドの clean な solve
   (中央値) を別プロセスで採り venv_base と一致することを確認 (afiro 0.120 vs 0.129 ms、kb2 0.328 vs 0.331)。
   既存の `ENOMOTO_PROF_PRESOLVE` は段階ごとに eprintln するので 0.1 ms 級では使えない (afiro: 0.9 ms vs 実 0.12 ms)。
2. **命令数**: 同ビルドを `CARGO_PROFILE_RELEASE_DEBUG=1` で作り
   `valgrind --tool=callgrind --toggle-collect='*PyModel*solve*'` で afiro/kb2/beaconfd/bore3d/standata/
   stocfor1/scorpion (各 3 solve)。負荷に依存しないので関数・行への帰属に使った。
3. **getenv 回数**: `LD_PRELOAD` で libc `getenv` を名前別に数える shim (`getenv_names.c`)。
4. **パスの費用対効果**: `ENOMOTO_PS_SKIP=<pass>` 等の実験用 env (計装ビルドのみ) と既存 `ENOMOTO_DISABLE_*`
   で 1 パスずつ切り替え、全 93 問を同じ手順で計測して基準 `base3.json` と比較 (§4)。同一ビルド同士の
   ノイズ: standmps/lotfi/agg3/scsd6 は基準側が高く出ており、どの変種でも -20〜-40% と出る → これらは
   ノイズ。「サイズ/反復が変わった問題」の列が実効果の目安。目的値・status は全変種・全問で基準と一致。
5. **round ごとの縮約量**: `ENOMOTO_PS_DEBUG_ROUNDS=1` で外側 round ごとに A 行数 / 実不等式行数 / 固定列数 /
   postsolve ログ長 / 所要時間を記録。

## 1. 全 93 問の時間内訳 (solve 昇順)

列: n→n' (構造列 入→出)、m→m' (行 入→出)、solve (計装なし中央値 ms)、presolve (`run_extended` 全体、最小値
ms)、以下 solve 比 %。reduce_eq / aggregator / scaling / red_ineq(round) は presolve の内数。構築 = `build_a_g`
+ `build_std_form_presolved` の presolve 以外 (行組み立て・shift/compaction・`freeze_std_matrices`)。起動 =
`solve_lp_dual_extended` 入口から主ループ開始まで (ColCache/crash/初回 LU 込み)。主ループ = 拡張双対の反復
ループ (`wall_t0` から `finish` 呼び出しまで)、finish = `finish` (polish / primal handoff)、postsolve =
`unscale_result` + 解放、その他 = 残り (`solve_mip` 分岐、目的値計算、env 判定、計測用の 1 行出力)。

| 問題 | n→n' | m→m' | solve ms (中央値) | presolve ms | presolve% | reduce_eq% | aggregator% | scaling% | red_ineq(round)% | 構築(std_form)% | 起動(setup+LU0)% | 単体法主ループ% | finish% | postsolve% | その他% | 反復 | round |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| afiro | 32→14 | 27→11 | 0.129 | 0.118 | 71.9 | 5.4 | 8.7 | 4.9 | 6.9 | 4.7 | 6.9 | 6.7 | 3.3 | 0.4 | 5.5 | 7 | 4 |
| sc50a | 48→28 | 50→29 | 0.221 | 0.147 | 56.9 | 9.2 | 8.4 | 5.4 | 5.2 | 4.3 | 6.2 | 20.3 | 3.1 | 0.3 | 8.0 | 24 | 3 |
| sc50b | 48→28 | 50→28 | 0.223 | 0.150 | 58.2 | 10.2 | 8.4 | 5.0 | 4.0 | 4.4 | 6.6 | 22.5 | 3.6 | 0.3 | 3.8 | 29 | 3 |
| kb2 | 41→28 | 43→36 | 0.370 | 0.193 | 51.5 | 5.3 | 8.8 | 4.7 | 3.8 | 3.7 | 5.3 | 28.6 | 3.3 | 0.3 | 7.0 | 33 | 3 |
| sc105 | 103→61 | 105→62 | 0.613 | 0.310 | 48.0 | 5.7 | 9.6 | 3.8 | 3.4 | 3.0 | 4.0 | 39.3 | 2.5 | 0.2 | 2.6 | 64 | 3 |
| recipe | 180→68 | 91→46 | 0.674 | 0.590 | 81.0 | 11.9 | 9.9 | 7.3 | 7.4 | 3.9 | 3.4 | 6.6 | 1.8 | 0.2 | 2.9 | 25 | 4 |
| blend | 83→53 | 74→48 | 0.830 | 0.411 | 49.8 | 9.4 | 9.8 | 4.1 | 2.7 | 2.7 | 2.8 | 36.3 | 2.2 | 0.2 | 4.4 | 64 | 3 |
| stocfor1 | 111→50 | 117→42 | 0.853 | 0.718 | 81.1 | 14.7 | 17.8 | 3.8 | 9.1 | 2.6 | 2.9 | 8.9 | 1.6 | 0.2 | 2.3 | 18 | 7 |
| adlittle | 97→94 | 56→53 | 0.879 | 0.302 | 33.7 | 4.0 | 5.2 | 3.6 | 3.2 | 2.4 | 3.1 | 55.3 | 2.2 | 0.1 | 2.9 | 83 | 3 |
| vtp.base | 203→39 | 198→35 | 0.940 | 0.864 | 66.0 | 21.4 | 10.5 | 7.1 | 8.7 | 3.8 | 1.9 | 6.6 | 1.7 | 0.2 | 18.5 | 26 | 6 |
| scagr7 | 140→67 | 129→64 | 1.053 | 0.651 | 68.1 | 25.3 | 7.9 | 3.7 | 3.7 | 3.1 | 3.1 | 18.6 | 2.1 | 0.2 | 3.9 | 46 | 3 |
| share2b | 79→72 | 96→86 | 1.150 | 0.425 | 35.0 | 2.1 | 7.9 | 4.0 | 3.0 | 3.1 | 2.8 | 52.0 | 2.5 | 0.1 | 3.9 | 84 | 3 |
| bore3d | 315→44 | 233→21 | 1.510 | 1.366 | 87.8 | 26.3 | 11.6 | 8.0 | 5.5 | 2.9 | 1.5 | 3.6 | 0.8 | 0.2 | 3.1 | 19 | 4 |
| scorpion | 358→74 | 388→60 | 1.541 | 1.416 | 80.0 | 21.4 | 15.2 | 6.5 | 5.7 | 3.1 | 1.6 | 10.4 | 1.1 | 0.2 | 3.2 | 61 | 4 |
| boeing2 | 143→139 | 185→139 | 1.697 | 0.552 | 31.5 | 0.8 | 3.2 | 4.9 | 2.8 | 2.7 | 2.6 | 59.1 | 1.9 | 0.1 | 1.9 | 113 | 3 |
| sc205 | 203→115 | 205→116 | 2.044 | 0.699 | 34.4 | 3.9 | 8.4 | 2.6 | 2.4 | 1.9 | 1.9 | 55.1 | 1.4 | 0.1 | 2.2 | 137 | 3 |
| beaconfd | 262→80 | 173→26 | 2.176 | 1.958 | 91.1 | 50.0 | 4.9 | 8.9 | 3.4 | 2.6 | 1.3 | 1.6 | 0.6 | 0.2 | 2.4 | 22 | 3 |
| agg | 163→105 | 488→160 | 2.212 | 1.496 | 59.2 | 4.2 | 13.0 | 6.2 | 6.5 | 3.0 | 2.2 | 28.7 | 1.3 | 0.1 | 5.2 | 75 | 8 |
| lotfi | 308→226 | 153→114 | 2.242 | 1.465 | 62.6 | 27.2 | 7.2 | 3.5 | 2.7 | 3.3 | 1.8 | 25.5 | 2.3 | 0.1 | 3.2 | 115 | 3 |
| share1b | 225→187 | 117→91 | 2.323 | 1.235 | 58.7 | 21.4 | 8.2 | 3.8 | 3.4 | 2.1 | 1.8 | 33.6 | 1.7 | 0.1 | 1.9 | 106 | 4 |
| capri | 353→194 | 271→136 | 2.723 | 1.762 | 58.7 | 11.1 | 11.2 | 4.6 | 4.4 | 2.5 | 1.6 | 33.5 | 1.1 | 0.1 | 2.3 | 128 | 4 |
| israel | 142→141 | 174→163 | 2.858 | 0.604 | 30.6 | 0.0 | 3.1 | 5.8 | 2.3 | 3.2 | 2.8 | 59.6 | 1.9 | 0.0 | 1.6 | 89 | 2 |
| standata | 1075→869 | 359→228 | 3.313 | 2.855 | 85.4 | 14.6 | 10.9 | 7.6 | 9.2 | 4.4 | 2.5 | 4.9 | 1.2 | 0.2 | 1.2 | 30 | 4 |
| agg2 | 302→281 | 516→283 | 3.437 | 2.000 | 57.8 | 4.1 | 8.6 | 6.6 | 4.1 | 3.2 | 2.4 | 32.8 | 1.7 | 0.1 | 1.8 | 145 | 4 |
| standgub | 1184→869 | 361→228 | 3.468 | 2.939 | 83.7 | 14.7 | 10.6 | 8.0 | 9.0 | 5.9 | 2.4 | 4.6 | 1.1 | 0.2 | 1.9 | 30 | 4 |
| agg3 | 302→276 | 516→285 | 3.661 | 2.044 | 54.7 | 3.6 | 8.5 | 5.7 | 3.7 | 3.9 | 2.3 | 35.6 | 1.6 | 0.1 | 1.5 | 159 | 4 |
| standmps | 1075→923 | 467→330 | 4.168 | 2.958 | 72.2 | 13.8 | 11.0 | 6.1 | 6.3 | 3.0 | 2.3 | 17.5 | 2.3 | 0.1 | 2.3 | 104 | 4 |
| seba | 1028→121 | 522→226 | 4.295 | 3.219 | 75.8 | 15.4 | 8.9 | 8.5 | 5.9 | 3.2 | 1.5 | 16.1 | 0.8 | 0.2 | 2.3 | 109 | 4 |
| sctap1 | 480→339 | 300→269 | 4.721 | 1.244 | 26.6 | 3.8 | 2.6 | 2.4 | 3.2 | 2.5 | 1.4 | 66.4 | 1.3 | 0.1 | 1.3 | 235 | 5 |
| finnis | 614→410 | 497→356 | 4.820 | 2.017 | 39.5 | 1.5 | 3.4 | 4.3 | 3.7 | 2.6 | 1.8 | 51.8 | 1.4 | 0.1 | 2.7 | 239 | 4 |
| brandy | 249→161 | 220→94 | 4.852 | 2.406 | 49.7 | 20.9 | 8.4 | 3.2 | 2.0 | 1.5 | 1.0 | 41.1 | 1.4 | 0.0 | 4.9 | 174 | 4 |
| scagr25 | 500→251 | 471→248 | 5.135 | 2.021 | 41.3 | 5.1 | 8.0 | 2.5 | 5.4 | 1.5 | 1.3 | 51.6 | 0.8 | 0.1 | 3.2 | 220 | 7 |
| fit1d | 1026→1024 | 24→24 | 5.689 | 3.544 | 61.9 | 0.5 | 13.6 | 14.6 | 4.3 | 5.9 | 2.4 | 26.2 | 1.4 | 0.1 | 2.0 | 64 | 2 |
| ship04s | 1458→1263 | 402→213 | 5.733 | 2.569 | 44.7 | 5.2 | 7.8 | 5.9 | 4.2 | 2.9 | 1.8 | 47.5 | 1.4 | 0.1 | 1.4 | 343 | 3 |
| tuff | 587→374 | 333→140 | 5.739 | 3.784 | 65.4 | 20.7 | 11.8 | 4.9 | 3.2 | 2.2 | 1.2 | 29.1 | 1.2 | 0.1 | 0.6 | 140 | 4 |
| gfrd-pnc | 1092→792 | 616→326 | 5.991 | 2.454 | 42.7 | 6.6 | 6.5 | 5.3 | 2.5 | 2.8 | 1.6 | 49.9 | 2.0 | 0.1 | 1.0 | 285 | 3 |
| bandm | 472→198 | 305→166 | 6.488 | 2.648 | 39.9 | 6.0 | 7.7 | 3.0 | 3.7 | 1.3 | 0.9 | 55.8 | 1.1 | 0.1 | 0.8 | 246 | 6 |
| shell | 1775→1214 | 536→240 | 6.521 | 3.725 | 59.0 | 7.2 | 8.2 | 6.7 | 4.1 | 2.9 | 1.5 | 33.1 | 1.9 | 0.1 | 1.0 | 257 | 3 |
| scfxm1 | 457→361 | 330→243 | 7.022 | 2.474 | 42.5 | 7.4 | 8.6 | 3.7 | 3.0 | 2.7 | 1.2 | 49.0 | 1.8 | 0.1 | 2.5 | 267 | 4 |
| scsd1 | 760→750 | 77→77 | 7.560 | 2.170 | 59.7 | 33.1 | 4.0 | 4.6 | 1.2 | 2.1 | 1.5 | 29.0 | 6.4 | 0.1 | 0.9 | 104 | 2 |
| scrs8 | 1169→800 | 490→192 | 7.796 | 3.266 | 51.4 | 5.9 | 10.0 | 4.5 | 5.3 | 2.2 | 1.3 | 41.6 | 1.6 | 0.1 | 1.6 | 236 | 4 |
| ship04l | 2118→1911 | 402→313 | 8.207 | 3.298 | 37.9 | 4.4 | 5.3 | 5.8 | 3.2 | 2.7 | 1.8 | 54.3 | 1.1 | 0.1 | 1.4 | 462 | 2 |
| scsd6 | 1350→1342 | 147→147 | 8.216 | 2.454 | 31.4 | 8.1 | 2.6 | 3.0 | 0.7 | 1.8 | 1.1 | 59.6 | 5.1 | 0.0 | 0.5 | 270 | 2 |
| fffff800 | 854→567 | 524→279 | 8.568 | 4.213 | 52.0 | 19.3 | 8.9 | 4.2 | 2.1 | 2.1 | 1.4 | 38.7 | 3.1 | 0.1 | 2.4 | 234 | 3 |
| boeing1 | 384→366 | 440→339 | 8.605 | 1.402 | 16.9 | 0.5 | 2.0 | 2.9 | 1.6 | 1.8 | 1.1 | 78.4 | 0.9 | 0.0 | 0.8 | 340 | 3 |
| e226 | 282→250 | 223→152 | 9.514 | 1.570 | 27.8 | 2.9 | 5.8 | 3.1 | 2.2 | 1.5 | 1.1 | 65.7 | 1.3 | 0.0 | 2.4 | 249 | 4 |
| forplan | 421→378 | 162→114 | 10.124 | 3.542 | 53.2 | 14.0 | 12.2 | 3.3 | 3.8 | 1.7 | 1.0 | 42.0 | 1.0 | 0.0 | 0.8 | 221 | 7 |
| ship08s | 2387→1588 | 778→284 | 10.394 | 4.018 | 41.2 | 4.8 | 6.0 | 5.6 | 4.3 | 2.5 | 1.4 | 52.4 | 1.0 | 0.1 | 1.4 | 512 | 3 |
| grow7 | 301→260 | 140→175 | 10.703 | 2.112 | 19.5 | 10.3 | 1.8 | 1.3 | 0.5 | 1.0 | 0.6 | 76.4 | 0.9 | 0.0 | 1.4 | 332 | 2 |
| ganges | 1681→491 | 1309→490 | 11.822 | 6.050 | 49.8 | 11.2 | 7.4 | 3.9 | 3.6 | 2.2 | 1.0 | 42.5 | 1.1 | 0.1 | 2.7 | 403 | 4 |
| ship12s | 2763→1924 | 1151→344 | 12.067 | 5.243 | 43.2 | 6.3 | 6.2 | 5.5 | 3.9 | 2.6 | 1.4 | 50.2 | 0.9 | 0.1 | 1.3 | 537 | 3 |
| etamacro | 688→421 | 400→289 | 12.161 | 1.844 | 22.4 | 3.9 | 3.3 | 2.5 | 1.9 | 1.6 | 1.0 | 63.3 | 7.8 | 0.1 | 3.6 | 349 | 3 |
| stair | 467→274 | 356→245 | 12.405 | 2.769 | 25.3 | 3.1 | 6.6 | 1.9 | 2.0 | 1.1 | 0.7 | 69.9 | 1.2 | 0.0 | 1.6 | 302 | 5 |
| sctap2 | 1880→1326 | 1090→977 | 12.933 | 3.650 | 29.2 | 1.5 | 3.2 | 3.4 | 3.4 | 2.2 | 1.6 | 65.0 | 1.2 | 0.1 | 0.4 | 329 | 4 |
| sierra | 2036→1748 | 1227→876 | 14.402 | 6.394 | 45.6 | 4.5 | 8.8 | 5.0 | 5.7 | 2.6 | 1.6 | 47.8 | 1.1 | 0.1 | 0.6 | 388 | 4 |
| scfxm2 | 914→733 | 660→495 | 15.230 | 4.446 | 29.9 | 4.5 | 5.7 | 2.4 | 2.2 | 1.3 | 0.9 | 65.7 | 1.2 | 0.0 | 0.6 | 573 | 4 |
| degen2 | 534→471 | 444→378 | 17.073 | 3.459 | 20.6 | 8.6 | 2.4 | 1.4 | 0.9 | 0.8 | 0.7 | 76.3 | 0.9 | 0.0 | 0.5 | 505 | 3 |
| czprob | 3523→2457 | 929→463 | 18.882 | 6.692 | 36.2 | 5.1 | 4.6 | 5.1 | 3.2 | 2.3 | 1.0 | 58.2 | 1.2 | 0.1 | 0.9 | 771 | 3 |
| modszk1 | 1620→828 | 687→439 | 20.522 | 7.697 | 38.6 | 4.6 | 6.6 | 1.7 | 3.2 | 0.8 | 0.6 | 58.7 | 0.8 | 0.1 | 0.3 | 528 | 8 |
| ship08l | 4283→3149 | 778→520 | 21.357 | 6.791 | 33.5 | 3.4 | 6.0 | 5.0 | 4.4 | 2.1 | 1.3 | 60.2 | 0.8 | 0.1 | 1.7 | 816 | 3 |
| sctap3 | 2480→1767 | 1480→1344 | 22.557 | 5.955 | 24.8 | 1.0 | 2.9 | 2.4 | 3.2 | 1.6 | 1.4 | 69.1 | 0.9 | 0.1 | 2.2 | 504 | 4 |
| stocfor2 | 2031→958 | 2157→1072 | 27.248 | 8.326 | 30.6 | 8.3 | 5.4 | 2.1 | 2.5 | 1.3 | 0.8 | 65.3 | 0.7 | 0.1 | 0.9 | 626 | 4 |
| pilot4 | 1000→743 | 410→322 | 27.655 | 5.590 | 20.4 | 2.5 | 3.9 | 1.2 | 1.4 | 0.8 | 0.4 | 77.1 | 0.6 | 0.0 | 0.6 | 571 | 6 |
| cycle | 2857→1783 | 1903→906 | 28.879 | 15.321 | 53.3 | 8.5 | 12.7 | 5.2 | 3.5 | 3.1 | 1.0 | 31.1 | 9.2 | 0.1 | 1.8 | 318 | 5 |
| scfxm3 | 1371→1098 | 990→742 | 29.039 | 6.191 | 21.8 | 4.9 | 4.5 | 1.7 | 1.4 | 1.1 | 0.6 | 74.1 | 0.9 | 0.0 | 1.1 | 887 | 4 |
| ship12l | 5427→4224 | 1151→686 | 29.368 | 10.284 | 34.3 | 3.8 | 6.0 | 4.2 | 4.7 | 2.1 | 1.1 | 60.6 | 0.7 | 0.0 | 0.9 | 1012 | 3 |
| maros | 1443→847 | 846→512 | 32.938 | 8.433 | 26.7 | 3.2 | 5.7 | 2.2 | 2.5 | 0.9 | 0.5 | 70.2 | 0.6 | 0.0 | 0.9 | 762 | 7 |
| bnl1 | 1175→981 | 643→455 | 33.352 | 4.859 | 15.3 | 2.3 | 2.5 | 1.2 | 1.1 | 0.6 | 0.5 | 82.7 | 0.6 | 0.0 | 0.2 | 943 | 4 |
| grow15 | 645→580 | 300→359 | 47.859 | 2.414 | 5.2 | 1.0 | 0.8 | 0.6 | 0.2 | 0.4 | 0.3 | 93.5 | 0.5 | 0.0 | 0.1 | 981 | 2 |
| fit1p | 1677→1028 | 627→627 | 51.355 | 4.865 | 9.5 | 2.5 | 0.5 | 1.1 | 0.4 | 0.7 | 0.4 | 88.2 | 0.2 | 0.1 | 1.0 | 893 | 2 |
| scsd8 | 2750→2744 | 397→397 | 52.635 | 4.479 | 9.0 | 3.0 | 1.2 | 1.2 | 0.3 | 0.7 | 0.4 | 86.8 | 0.5 | 0.0 | 2.5 | 1145 | 2 |
| woodw | 8405→4006 | 1098→551 | 57.561 | 32.783 | 59.6 | 15.3 | 7.8 | 4.4 | 10.5 | 2.2 | 0.6 | 35.2 | 0.7 | 0.1 | 1.6 | 659 | 5 |
| perold | 1376→1065 | 625→490 | 60.727 | 6.653 | 11.4 | 2.5 | 2.3 | 0.7 | 0.9 | 0.5 | 0.3 | 86.0 | 0.4 | 0.0 | 1.3 | 1135 | 6 |
| bnl2 | 3489→2224 | 2324→1061 | 67.040 | 20.682 | 30.6 | 1.5 | 7.5 | 1.6 | 4.2 | 1.1 | 0.6 | 65.6 | 1.4 | 0.0 | 0.6 | 1029 | 9 |
| nesm | 2923→2255 | 750→1140 | 76.733 | 10.820 | 14.5 | 1.0 | 2.8 | 1.3 | 1.2 | 0.9 | 0.5 | 77.4 | 6.2 | 0.1 | 0.3 | 1196 | 3 |
| d6cube | 6184→5446 | 415→401 | 82.969 | 27.149 | 33.6 | 9.3 | 5.8 | 4.2 | 1.4 | 1.9 | 0.7 | 60.5 | 0.8 | 0.0 | 2.5 | 420 | 3 |
| pilotnov | 2172→1648 | 975→708 | 87.803 | 22.909 | 26.8 | 2.9 | 8.1 | 1.0 | 3.0 | 0.7 | 0.3 | 71.7 | 0.3 | 0.0 | 0.1 | 1115 | 15 |
| wood1p | 2594→1800 | 244→169 | 88.974 | 55.480 | 65.9 | 46.5 | 5.4 | 3.9 | 0.6 | 2.0 | 0.5 | 30.2 | 0.4 | 0.0 | 1.0 | 307 | 3 |
| fit2d | 10500→10426 | 25→25 | 101.542 | 54.701 | 49.4 | 0.3 | 12.9 | 9.7 | 3.2 | 4.7 | 1.3 | 36.1 | 0.8 | 0.6 | 6.9 | 143 | 2 |
| pilot.we | 2789→2304 | 722→561 | 115.796 | 14.511 | 13.9 | 1.2 | 3.3 | 0.6 | 1.5 | 0.4 | 0.2 | 85.1 | 0.3 | 0.0 | 0.1 | 2193 | 10 |
| pilot.ja | 1988→1310 | 940→646 | 135.024 | 13.829 | 11.7 | 2.3 | 2.6 | 0.7 | 0.9 | 0.4 | 0.2 | 87.1 | 0.5 | 0.0 | 0.0 | 1450 | 7 |
| grow22 | 946→860 | 440→520 | 147.690 | 3.478 | 2.8 | 0.5 | 0.4 | 0.4 | 0.1 | 0.3 | 0.1 | 96.5 | 0.2 | 0.0 | 0.0 | 2047 | 2 |
| greenbea | 5405→3077 | 2392→1238 | 180.751 | 36.867 | 21.0 | 2.2 | 4.4 | 1.7 | 1.2 | 0.9 | 0.3 | 74.1 | 3.2 | 0.1 | 0.2 | 1918 | 4 |
| 80bau3b | 9799→8913 | 2262→1977 | 212.407 | 26.550 | 13.1 | 0.0 | 1.3 | 1.5 | 1.9 | 0.8 | 0.4 | 85.0 | 0.4 | 0.0 | 0.2 | 3076 | 3 |
| 25fv47 | 1571→1416 | 821→686 | 212.486 | 10.178 | 4.9 | 1.2 | 1.3 | 0.5 | 0.3 | 0.2 | 0.1 | 93.6 | 0.3 | 0.0 | 0.7 | 3094 | 5 |
| degen3 | 1818→1713 | 1503→1407 | 246.067 | 11.921 | 6.3 | 0.5 | 1.2 | 0.6 | 0.4 | 0.5 | 0.2 | 91.9 | 0.5 | 0.0 | 0.6 | 1849 | 3 |
| greenbeb | 5405→3078 | 2392→1245 | 419.837 | 45.260 | 10.2 | 0.8 | 2.5 | 0.6 | 0.6 | 0.3 | 0.1 | 89.2 | 0.2 | 0.0 | 0.1 | 4259 | 7 |
| pilot | 3652→3093 | 1441→1243 | 798.236 | 43.110 | 5.3 | 0.1 | 1.2 | 0.3 | 0.7 | 0.2 | 0.1 | 94.3 | 0.1 | 0.0 | 0.0 | 3081 | 10 |
| maros-r7 | 9408→4426 | 3136→2152 | 928.216 | 177.704 | 19.5 | 13.3 | 0.6 | 0.9 | 0.2 | 0.4 | 0.1 | 79.0 | 0.2 | 0.1 | 0.6 | 2442 | 3 |
| d2q06c | 5167→4184 | 2171→1898 | 1332.124 | 76.549 | 6.2 | 0.4 | 2.0 | 0.2 | 0.7 | 0.1 | 0.1 | 93.1 | 0.5 | 0.0 | 0.0 | 6529 | 16 |
| fit2p | 13525→10525 | 3000→3000 | 1417.773 | 30.495 | 2.1 | 0.4 | 0.1 | 0.3 | 0.1 | 0.2 | 0.1 | 97.4 | 0.1 | 0.0 | 0.1 | 5796 | 2 |
| pilot87 | 4883→4486 | 2030→1877 | 4703.314 | 76.948 | 1.7 | 0.0 | 0.4 | 0.1 | 0.2 | 0.1 | 0.0 | 89.5 | 8.5 | 0.0 | 0.1 | 6461 | 11 |
| dfl001 | 12230→8854 | 6071→3603 | 7994.124 | 90.038 | 1.1 | 0.5 | 0.2 | 0.1 | 0.0 | 0.0 | 0.0 | 97.7 | 0.0 | 0.0 | 1.2 | 20712 | 4 |

要約 (単純平均):

| 群 | 問題数 | presolve | 構築 | 起動+LU0 | 主ループ | finish | postsolve | その他 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| solve < 1 ms | 10 | 60.0% | 3.5% | 4.5% | 24.3% | 2.6% | 0.2% | 4.3% |
| solve < 3 ms | 21 | 58.9% | 3.2% | 3.2% | 27.6% | 2.0% | 0.2% | 4.3% |
| solve < 10 ms | 49 | 53.7% | 2.9% | 2.3% | 35.7% | 2.0% | 0.1% | 2.9% |
| solve < 50 ms | 70 | 46.4% | 2.5% | 1.9% | 44.7% | 1.8% | 0.1% | 2.3% |
| 全 93 問 | 93 | 39.4% | — | — | — | — | — | — |

**93 問の単純平均で presolve が solve 時間の 39%、solve < 10 ms の 49 問では 54%。** 幾何平均基準では
presolve の固定費削減が主ループ改善と同等以上に効く。

presolve の内訳 (light = solve < 10 ms の 49 問、その solve 合計 191.8 ms に対する比; `base3.json`):

| パス | 全 93 問 ms | light ms | light solve 比 | 呼出 | 縮約数 | 効いた問題数 |
|---|---:|---:|---:|---:|---:|---:|
| reduce_equalities | 291.0 | 18.5 | **9.6%** | 93 | 570 行 (566 は単純重複) | 20 (重複以外は 4 問) |
| aggregator (v2) | 190.5 | 14.5 | **7.5%** | 404 | 7654 列 | 70 (空振り 275/404 回) |
| scaling::compute + apply | 86.0 | 9.5 | **5.0%** | 93 | — | — |
| reduce_inequalities(round) | 78.7 | 7.0 | **3.7%** | 404 | 291 行 | 13 (80 問で 0) |
| propagate | 70.9 | 5.2 | 2.7% | 404 | 境界 | — |
| parallelcols | 59.7 | 6.1 | 3.2% | 222 | 1815 列 | 33 (空振り 185/222) |
| doubleton | 47.2 | 5.7 | 3.0% | 178 | 4181 列 | 67 |
| dualpropagate | 33.2 | 2.5 | 1.3% | 126 | 1443 | 19 |
| colsingleton | 28.9 | 2.6 | 1.4% | 404 | 14039 列 | 75 |
| eqprop | 22.9 | 2.6 | 1.4% | 186 | 境界 | — |
| rebuild_g(inner)+extract_bounds(inner) | 32.7 | 2.6 | 1.4% | 808 | — | — |
| foldfixed(A)+(G) | 18.9 | 1.4 | 0.7% | 808 | — | — |
| dualfix | 11.7 | 0.95 | 0.5% | 404 | 160465 列 | 88 |
| reduce_inequalities (ループ前) | 14.6 | 1.7 | 0.9% | 93 | 417 行 | 33 |
| final propagate | 11.8 | 1.1 | 0.6% | 93 | — | — |
| rowsingleton | 4.5 | 0.5 | 0.25% | 404 | 37 | 11 |
| freevar | 3.4 | 0.2 | 0.1% | 93 | 44 | 2 |
| build_a_g (presolve 外) | 25.3 | 2.5 | 1.3% | 93 | — | — |
| 標準形構築 rows+freeze (presolve 外) | 15.4 | 1.6 | 0.9% | 93 | — | — |

- 外側 round: 404 round 中 **201 round は行数・固定列数・postsolve ログのどれも変わらない空回り**で、round
  ループ時間 808 ms のうち 242 ms (30%)。d2q06c 14/16 round (41 ms)、pilot87 9/11 (32 ms)、pilotnov 13/15
  (17 ms)、pilot 6/10 (14 ms)、greenbeb 4/7 (10 ms)、maros 5/7 (7 ms)、scfxm3 2/4 (6 ms)。
- 命令数 (callgrind、afiro = 1.21 M 命令/solve): `run_extended` 81%、**`std::env::var` 12.7%** (libc
  `getenv` 10.6%)、propagate 11.4% (内 rebuild_g_ref 8.6%、extract_bounds 6.3%)、reduce_inequalities 9.9%
  (内 `HashMap::insert` 5.5% + `reserve_rehash` 4.2%)、aggregator 7.9%、parallelcols 6.9%、scaling 6.8%、
  doubleton 6.7%、`csr_from_rows` 5.1%、malloc 系 (`mi_malloc_aligned` 7.9% + `grow_one` 7.5% +
  `finish_grow` 6.8% + realloc 3.5% + free 3.8%) ≈ 25%、`solve_lp_dual_extended` 全体で 13.5%。
  beaconfd: reduce_equalities 44.5% (faer `qr_in_place` 32%、dedupe の `HashSet<Vec>`+SipHash ≈ 9%)。
  bore3d: `drop_linearly_dependent_sparse` 18.8% (BTreeMap/BTreeSet)。standata: aggregator 12.5%、
  reduce_inequalities 11.3%、reduce_equalities 11.2%、parallelcols 8.0%、propagate 7.8%+rebuild_g 7.7%、
  scaling 7.1%、主ループ 8.7%。stocfor1: aggregator 18.5%、reduce_eq 12.5%、reduce_ineq 11.0%。
- 初回呼び出し (新プロセス 1 回目) は 2 回目以降より中央値 +2.2 ms (afiro +0.29、kb2 +0.40、adlittle +4.4、
  bandm +8.8 ms; ページイン・mimalloc 初期化・rayon 生成)。`ab_bench.py` は 25 ms 未満を 20 回解いた中央値
  なので採用判定には効かない。

## 2. 原因 (file:line)

### 2.1 `reduce_equalities`: 89/93 問で「重複行除去以外は何も見つけない」のに毎回 QR / 疎ガウス消去 (light 9.6%)
- `src/presolve.rs:409` → `src/presolve/redundancy.rs:133-166`。密経路 (`density > DENSE_DENSITY_THRESHOLD =
  0.03`, `:111,149`) は `(n+1)×p` 密行列 + faer 列ピボット QR (`:199-260`)。疎経路 `drop_linearly_dependent_sparse`
  (`:386-790`) は**行を `BTreeMap<usize,f64>`、列→行を `BTreeSet`、ピボット候補を `BTreeSet<(u64,usize)>` の
  heap で持つ**消去 (`:392-420`, `:430-440 refresh_col`)。300 行以上は Dulmage–Mendelsohn 分解 (`:901-960`)、
  成分合計 64 行以上で **rayon `par_iter`** (`:957-960`; 22 問で発火、新プロセスではここで rayon プール生成)。
- 実測: 削除 570 行のうち 566 行は `dedupe_rows` (`:170-190`) の単純重複。消去本体が落とした行は scorpion 30、
  degen2 2、bore3d 1、wood1p 1 の **4 問だけ**。残り 89 問で 249.5 ms (light 17.7 ms) が純粋な無駄 =
  幾何平均 **-7.4%** 相当 (beaconfd 50%、scsd1 33%、lotfi 27%、scagr7 25%、share1b/vtp.base 21%、brandy/tuff 21%、
  fffff800 19%、seba 15%、maros-r7 14% = 127 ms、wood1p 47% = 40 ms)。
- §4: 丸ごとスキップ (`noredeq`) で幾何平均 **-9.5%**、10% 超退行 1 (agg2 +48%: サイズ・反復同一なので
  ノイズ)、目的値不一致 0/93、最終縮約サイズが変わるのは wood1p (169→170 行, 反復 307→313) と degen2
  (378→380 行, 505→500) の 2 問だけ。scorpion の従属 30 行は他パスが結局消している (最終 74×60 同一)。

### 2.2 aggregator v2 が空振りでも A・実行行・b・c と列索引 6 本を毎 round 作り直す (light 7.5%)
- `src/presolve.rs:918` → `src/presolve/aggregator.rs:893-935`: `csr_rows(a)`、`real_rows.to_vec()`、
  `b/c.to_vec()`、`a_col_idx/g_col_idx/a_col0/g_col0: Vec<Vec<_>>` ×4 (各 n 本)、`act_a/act_g/row_max_a/stamp`
  を候補ゼロでも無条件に確保。**P2 (採用済み) の早期 return は row-local 版 `eliminate_implied_free_columns_if_any`
  (`:328`) にしかなく、v2 が既定 (stocfor2 対策) になった時点で失われた。**
- 実測: 404 呼出中 275 空振り (68%)。全呼出が空振りの 23 問で 38.5 ms (fit1d 13.6%、fit2d 12.9%、seba 8.9%、
  ship04s 7.8%、woodw 7.8%、ship12s/12l/08l 6%)。kb2 で命令数 11.7% (3 呼出中 2 空振り)。空振り 1 回 =
  n=97 で 16 µs、n=2594 で 1.5 ms、n=10500 で 5.8 ms。

### 2.3 solve 1 回あたり `std::env::var` 95〜370 回 (afiro の命令数 12.7%)
- LD_PRELOAD 実測: afiro 99、kb2 95、bandm 372 回/solve。bandm の内訳: `ENOMOTO_KEEP_IMPLIED_BOUND_ROWS`
  **180 回** (`colsingleton.rs:78-80` `skip_implied_bound_rows()` が `colsingleton.rs:217`・`doubleton.rs:164` で
  候補列ごとに呼ばれる)、`ENOMOTO_DEBUG_XB_DRIFT_EXT` 49 回 (主ループ側、担当外だが同種)、round ごとに
  `ENOMOTO_INEQ_SINGLETON` (`presolve.rs:636`)、`DISABLE_AGGREGATOR`/`XROW`/`ROWLOCAL` (`:908-912`)、
  `AggOptions::from_env` 4 本 (`aggregator.rs:870-878`)、`DEBUG_AGGREGATOR` (`:921`)、`DISABLE_PARALLELCOLS`/
  `DEBUG_PARALLELCOLS` (`:1004,1010`)、`FIXPOINT_EXACT` (`:1089`、クロージャ内で round ごと)、solve ごとに
  `simplex.rs:990,3072,3116,3117,3132`、`extended_dual.rs:2436,2493,2514` など (`ENOMOTO_*` 全 87 名)。
- 1 回のコストは環境変数の本数に比例 (libc `getenv` は線形探索): この環境 (145 変数, 7.7 KB) で 116 ns、
  空環境で 6.7 ns、Rust 側ロック+`OsString` 込み ≈ 150 ns → afiro ≈ 15 µs (solve の ~12%)、kb2 ≈ 14 µs
  (4%)、bandm ≈ 55 µs (0.9%)。環境変数を 400 本足すと afiro の最小時間 0.101→0.125 ms。ベンチ環境の
  シェルの env 次第で結果が動く要因でもある。

### 2.4 `reduce_inequalities(round)`: 境界行 2n 本まで毎 round ハッシュし、HashMap を容量なしで作る (light 3.7%)
- `src/presolve.rs:1076` → `redundancy.rs:1314-1420`。G の行の大半は `rebuild_g_ref` が変数ごとに作る単項の
  境界行で、同士では重複し得ない (P7 と同じ指摘)。`heads = HashMap::default()` (`:1341`) が容量なしで、
  afiro では `reserve_rehash` 4.2% + `insert` 5.5%。
- 実測: 80 問で削除 0 (67.9 ms; woodw 10.5%、standata 9.2%、standgub 9.0%、vtp.base 8.7%、recipe 7.4%、afiro
  6.9%、agg 6.5%)。削除があるのは 13 問 (seba 147、ganges 85、sierra 21、greenbea 11、…)。§4: 丸ごとスキップで
  幾何平均 **-4.7%**、サイズ/反復が変わるのは 5 問 (maros-r7 +12% が唯一の 10% 超退行)。

### 2.5 スケーリングが境界行 2n 本込みの G に 10 反復 (light 5.0%)
- `src/presolve/scaling.rs:133-180` (`compute`)、`:189-205` (`apply`)。`build_a_g` (`presolve.rs:155-164`) が
  有限境界ごとに単項行を G に入れるため、毎反復 A + G(実行 + 2n 単項) を faer イテレータ経由で 2 回走査。
  fit2d (n=10500) では単項行が G の 99%。単項行の `e_g[i]` は `d[j]` だけで決まる (`1/sqrt(|d_j|)`)。
- 実測: compute 68.3 ms + apply 17.8 ms。fit1d 12.7%、fit2d 7.9%、beaconfd 7.6%、seba 6.6%、bore3d 6.5%、
  standgub 6.4%、standata 5.9%。kb2 命令数 6.0%。§4: 反復 0 (スケーリングなし) は 23 問の部分計測で退行 9 問
  (capri +30%, boeing1 +28%, cycle +27%)・dfl001 が完走せず → スケーリング自体は必要。反復数 3/5 と境界行
  除外 (`ENOMOTO_PS_SCALE_NOBOUNDS`) は未計測。

### 2.6 外側 round の半分が空回り (round ループ時間の 30%)
- `src/presolve.rs:509-1099`。署名 (`:1080`) が lb/ub を含み、`propagate` (`propagate.rs:271-283`) は絶対 1e-9
  より大きい締め付けをすべて採用して次 round の署名を変える。HiGHS は相対 1e-3 未満の改善は境界を変えない
  が、本実装は境界を変えたうえで比較だけ 1e-3 で緩める (`:1092`) ため、幾何級数的に縮む境界 (d2q06c,
  pilotnov, pilot87, pilot) では 20 round 上限まで `eqprop`/`dualfix`/`foldfixed`/`rowsingleton`/`colsingleton`/
  `aggregator`/`parallelcols`/`reduce_inequalities` を全部やり直す。

### 2.7 境界を「行」として毎 round 往復させる設計による malloc/コピー支配
- `propagate` (`propagate.rs:144-333`) は入口 `extract_bounds` (行→lb/ub + 実行 `Vec<Vec>` コピー)、出口
  `rebuild_g_ref` (lb/ub→単項行 2n 本 + CSR 再構築)。同じ往復が inner ループ (`presolve.rs:735`, `:833`)、
  aggregator 後 (`:941`)、parallelcols 後 (`:1030`)、最終 (`:1193`) にもある。doubleton (`doubleton.rs:93`)、
  colsingleton (`colsingleton.rs:113,126-136`)、dualpropagate (転置 CSC `dualpropagate.rs:188-204`)、
  parallelcols (`csr_rows` + CSC `parallelcols.rs:172-189`) も各自 A/G を丸ごとコピーして索引を作る。
- 実測: afiro で propagate+rebuild_g_ref+extract_bounds が命令数 20%、malloc 系 25%、`csr_from_rows`/
  `CsrRowBuilder` 10%。standata: propagate 7.8% + rebuild_g 7.7% + push_row 6.8% + csr_from_rows 6.7%。

### 2.8 parallelcols / doubleton / dualpropagate の空振り
- parallelcols: 222 呼出中 185 空振り、全空振りの 60 問で 18.3 ms (幾何平均 -1.3% 相当)。ラッチが 2 連続
  空振りなので最低 2 回は `csr_rows` + CSC 構築 (`parallelcols.rs:172-189`)。§4: 無効化で幾何平均 -3.9% だが
  29 問でサイズ/反復が変わり grow7 +50%, greenbea +23%, vtp.base +15% → 無効化ではなく空振りコスト削減が対象。
- doubleton: 26 問で 0 置換 (11.6 ms、fit2d 3.0%)。1 ストライクラッチでも必ず 1 回は全行 `rewrite_row` +
  `csr_from_rows` ×2 (`doubleton.rs:89-220`; P3)。§4: 無効化は pilot87 +421% → 必須パス。
- dualpropagate: 74 問で 0 (12.1 ms; sc50a/sc50b 3.7%)。§4: 無効化で幾何平均 -2.1% だが etamacro +27%,
  stocfor2/boeing2 +16% → 転置構築の再利用 (P11) と早期打ち切りが対象。

### 2.9 標準形構築・起動・その他 (solve < 1 ms で合計 12%)
- `build_a_g` (`presolve.rs:128-169`): `BTreeMap` 走査 → `Vec<Vec>` → `csr_from_rows` ×2 (1.3%)。
- `build_std_form_presolved` の行組み立て (`simplex.rs:1231-1273`: 行ごと `Vec::new()`+push、
  `freeze_std_matrices` `:760` で CsrMat/CscMat を別々に構築) 0.9%; shift/compaction ループ `:1075-1214`。
- `solve_lp_dual_extended` 起動 (`extended_dual.rs:2424-2530`: `perturb_costs`、`delta`、`ColCache::build` ×2
  (`:2457,2461`)、`crash` (`:2492`)、初回 `refactorize` (`:2526`)、15 本以上の `vec![0.0; m/n_total]`) —
  solve < 1 ms で 4.5% (afiro: setup 4.1 µs + LU 3.5 µs + 残り 9 µs)。
- 「その他」(< 1 ms で 4.3%): `solve_lp` の目的値計算 (`solver.rs:39-45`)、`PyDict`/list 変換
  (`model.rs:145-155`)、上記 env 判定、`drop` (afiro で 0.5 µs)。

### 2.10 その他の気付き
- `ext_finish` (polish → primal handoff、`extended_dual.rs:5069-5343`) は light 群 2.0%、pilot87 8.5%
  (400 ms)、cycle 9.2%、etamacro 7.8%、scsd1 6.4%、nesm 6.2% (前回 L13/L14、担当外)。
- `freevar` (`freevar.rs:206-215`) は自由変数 1 本消すごとに長さ n の `Vec<Vec<usize>>` を作り直す (O(n × 自由
  変数数))。Netlib では pilot.ja/pilot4 のみ、無害。
- `eqprop` を切ると +10.3% (woodw +172%, pilot +166%, greenbea +146%)、aggregator を切ると +7.7%
  (stocfor2 +211%) — この 2 パスは縮約効果が大きく、無効化は論外。ただし eqprop は 2 round 固定
  (`presolve.rs:549`)。

## 3. 対処法一覧

各項目: 対象 / 機序 / 期待効果 (幾何平均; 個別問題) / 経路 (縮約結果・数値経路) / リスク / 難易度 (S/M/L)。
「ビット同一」= `ENOMOTO_DEBUG_PRESOLVE_HASH` の縮約ハッシュが不変であることを設計で保証できるもの。

**C1. 従属等式検出を「重複除去 + 安価な判定」に限定する** (最大の項目)
- 対象: `redundancy.rs:133-166` (`reduce_equalities`)、呼び出し `presolve.rs:409`。
- 機序: (a) `dedupe_rows` は残し (566/570 行はこれで落ちる)、rank 判定 (`drop_linearly_dependent*`) は
  「密経路の O(p²n) と疎経路の BTree 消去」を既定で走らせない。代替: (i) 完全に外す (§4 `noredeq`:
  幾何平均 -9.5%、目的値不一致 0、退行なし。従属行が残ったのは wood1p/degen2 だけで反復 +2%/-1%)、
  (ii) 縮約後 (round ループの後、`presolve.rs:1101` 付近) の小さい A にだけ掛ける (C10 の不採用理由は
  「ship* で 4→17 ms」だが、それは dedupe も後ろに動かした結果; dedupe は前、rank 判定だけ後ろなら
  ship* の重複行は前で消える)、(iii) 行数 p ≤ 数十 or 単体法が singular basis を検出したときだけ実行。
- 効果: 幾何平均 -7〜-9.5%; beaconfd -50%、wood1p -44%、scsd1 -33%、lotfi -27〜40%、bore3d -34%、scagr7 -37%、
  maros-r7 -14%。
- 経路: (i)(ii) は wood1p/degen2/scorpion/bore3d の縮約結果が変わる (従属行が残る / 別パスが消す)。
  リスク: 従属等式が基底 LU で特異になる問題 (Netlib 93 問では発生せず全問正答) — 拡張双対の `None` 経路
  (`simplex.rs:3141`) が保険。難易度: (i) S、(ii) S〜M、(iii) M。

**C2. `dedupe_rows` を SipHash `HashSet<Vec<(usize,u64)>>` から FxHash/自前 mix + `Vec` 索引に** (ビット同一)
- 対象: `redundancy.rs:170-190`。行ごとに署名 Vec を確保し SipHash で挿入 (beaconfd 命令数 ≈ 9%)。
- 機序: `reduce_inequalities` (`:1336-1342`) と同じ「u64 ハッシュ → 同署名クラス連鎖 → 元の行を直接比較」に
  揃え、`HashMap::with_capacity(p)`。
- 効果: C1 と併用で reduce_equalities 残りをほぼゼロに (beaconfd 追加 -8%)。リスクなし。難易度 S。

**C3. aggregator v2 に候補ゼロの早期 return (P2 の v2 版)** (ビット同一)
- 対象: `aggregator.rs:893-935`、呼び出し `presolve.rs:918-919`。
- 機序: 候補条件 (`col_a_count >= min_a && col_a_count+col_g_count >= 2 && !free && implied range ⊆ box`,
  `:983-993`) は CSR と `real_rows` を借用したまま O(nnz) で判定できる。候補ゼロなら `None` を返し
  (`if_any` `:328` と同じ契約)、コピーも索引構築もしない。候補があるときだけ現行の v2 本体を呼ぶので
  結果は同一。計装 worktree に実装済み (`patch4.py` `v2_has_candidate`、`ENOMOTO_PS_AGG_PRECHECK`; 未計測)。
- 効果: 空振り 275 回分 → 幾何平均 -1.5〜-2.5%; fit1d -13%、fit2d -12%、seba -9%、ship* -6〜8%、woodw -8%。
  リスクなし。難易度 S。

**C4. `std::env::var` を solve 経路から追い出す** (ビット同一)
- 対象: `colsingleton.rs:78-80,217`、`doubleton.rs:164` (候補ごと)、`presolve.rs:600,636,908-921,1004-1010,1089`
  (round ごと)、`aggregator.rs:870-878`、`simplex.rs:990,3072,3116-3132`、`extended_dual.rs:2436,2493,2514`
  (solve ごと)。主ループ側の `ENOMOTO_DEBUG_XB_DRIFT_EXT` (49 回/solve) も同種。
- 機序: 全 `ENOMOTO_*` を `OnceLock<Config>` (プロセスで 1 回だけ読む) か、`run_extended` 入口で 1 回
  読んで `bool` を渡す。`skip_implied_bound_rows()` は関数の外で 1 回。
- 効果: afiro -10%、kb2 -4%、sc50a/sc50b -4%、bandm -1%; 幾何平均 -1.5〜-2%。ベンチ結果のシェル環境依存も消える。
  リスクなし (env 変更をプロセス途中で反映できなくなるだけ)。難易度 S。

**C5. `reduce_inequalities(round)` で単項行をハッシュ対象から外し、HashMap を容量付きで確保** (P7 の再提案)
- 対象: `redundancy.rs:1341` (容量)、`:1352-1400` (単項行)。
- 機序: 単項行 (境界行) は kept にそのまま通す (P7 実装は列ごとの連鎖で判定完全同一; ハッシュを
  `with_capacity(m)`)。P7 は「10% 改善が 1 問もない」で不採用だったが、新基準では削除 0 の 80 問で
  67.9 ms = 幾何平均 -2.6% 相当が対象 (sierra -9.4% など)。
- 効果: 幾何平均 -1.5〜-2.5%; woodw/standata/standgub/vtp.base -5〜9%。経路: P7 実装は同一 (要ハッシュ確認)。難易度 S。

**C6. 外側 round の空回りを止める: propagate の相対改善しきい値 + 「構造が変わらなければ即終了」**
- 対象: `propagate.rs:271-283` (`candidate < ub - EPS`)、`presolve.rs:1080-1098` (署名)。
- 機序: (a) HiGHS 流に相対 1e-3 未満の締め付けは境界を変えない (`ENOMOTO_PS_PROP_RELTOL` で実装済み、未計測)。
  (b) round の出口で「A 行数・実行数・固定列数・postsolve ログ長が前 round と同じ」なら、残り round は
  同じ決定的関数の再実行なので打ち切る (P5 とは違い propagate の出力ではなく構造で判定; d2q06c 14 round、
  pilotnov 13、pilot87 9 の空回りを 1 round に)。
- 効果: round ループ時間の 30% (242 ms); 幾何平均 -1〜-2%、d2q06c -3%、pilotnov -19%、pilot87 -0.7%、
  maros -20%、stair -9%、e226 -9%、share1b -16%。経路: (a) は縮約結果 (境界値) が変わる。(b) は境界が
  微小に変わり続けた分を捨てる = 縮約結果が変わり得る (2026-09-23 の「構造的不動点」案は pilot +10.6% で
  不採用: そのときは round 1 から構造だけで止めたが、ここでは「1 round 空回りしてから」止めるので別物。
  要 A/B)。難易度 S。

**C7. スケーリングから境界行を外す / 反復数を減らす** (経路変更)
- 対象: `scaling.rs:133-180`。`ENOMOTO_PS_SCALE_NOBOUNDS`/`ENOMOTO_PS_RUIZ` で実験可能 (未計測)。
- 機序: 単項行は `e_g` を `1/sqrt(|d_j|)` に閉形式で決められるので走査不要。反復 10→5 は収束状況による。
  HiGHS は縮約後に、境界行なしでスケーリングする。
- 効果: compute の 5〜50% (fit2d は G の 99% が単項行) → 幾何平均 -1〜-2%。リスク: スケール係数が変わり
  反復数が動く (Ruiz 0 反復では capri/boeing1/cycle +30%, dfl001 完走せず — 完全に外すのは不可)。難易度 S。

**C8. スケーリングの平坦ループ化 (P8 再掲、ビット同一)**
- 対象: `scaling.rs:56-64,133-180,189-205`。faer イテレータ (`col_indices_of_row().zip(values_of_row())`) を
  `row_ptr/col_ind/values` の平坦スライスに、`apply` は `CsrRowBuilder` に直接書く (行ごとの `Vec` 生成なし)。
- 効果: scaling 5.0% の 3〜4 割 → 幾何平均 -1〜-1.5%。難易度 S。

**C9. parallelcols の空振りコスト削減** (ビット同一)
- 対象: `parallelcols.rs:172-189` (`csr_rows` + `CscMat::from_entry_stream`)、ラッチ `presolve.rs:1013-1017`。
- 機序: 列署名は CSR を列方向に 1 パス転置するだけで作れる (`csr_rows` 不要)。A と実行が前 round から
  変わっていなければ (nnz・行数同一かつ他パスが空) 前回の署名を再利用。P4 は P3 と束ねて評価され
  「最大 -6.5%」で不採用 → 新基準では対象 (全空振り 60 問 18.3 ms = -1.3%)。難易度 S〜M。

**C10. doubleton の空振り早期 return (P3 再提案)** (ビット同一)
- 対象: `doubleton.rs:89-220`。候補走査 (`row.len()==2`) を CSR 行スライスで先に行い、`subs` が空なら入力を
  clone して返す。P3 は「最大 capri -6.9%」で不採用 → 新基準では対象 (0 置換 26 問 11.6 ms; fit2d -3%)。
  難易度 S。

**C11. dualpropagate の転置再利用と早期打ち切り (P11 再提案)**
- 対象: `dualpropagate.rs:188-204` (CSC 構築)、`:212-232` (双対行構築)。A/実行が不変の round は前回の
  `col_terms` を再利用; 双対問題に「上下どちらかが無限の列」が 1 本もなければ (`t_rows` が単項行のみ) 何も
  起きないので `propagate` を呼ばずに返す。効果: 0 縮約の 74 問 12.1 ms (-0.8%)。難易度 S。

**C12. inner ループの `rebuild_g` → `extract_bounds` 往復の省略 (P9 再提案、ビット同一)**
- 対象: `presolve.rs:735-737`, `:833-839`。`rs.fixes`・両置換が空なら g/h と lb/ub/実行は不変。
  効果 -0.5〜-1%。難易度 S。

**C13. 「境界を行にしない」表現への段階的移行 (P6 の続き、M〜L)**
- 対象: `propagate.rs` (`extract_bounds`/`rebuild_g_ref`)、`presolve.rs` の各 `rebuild_g`、`colsingleton.rs:126-136`、
  `doubleton.rs:93`、`redundancy.rs:1314` の呼び出し側。
- 機序: round 内は `(a: Csr, real_rows: Vec<Vec>, real_rhs, lb, ub)` を正とし、G (境界行込み CSR) は
  `propagate`/`reduce_inequalities` が必要とする瞬間だけ組む。C5/C6/C7/C9/C12 の多くが自然に不要になる。
- 効果: afiro で命令数の 20% (propagate 往復) + malloc 25% の相当部分 → 幾何平均 -3〜-6% (推定)。
  ビット同一にできる (順序を保てば)。難易度 M〜L。

**C14. `reduce_inequalities` のループ前呼び出しも境界行を除外** (C5 と同じ、`presolve.rs:421`)。効果 -0.3%。S。

**C15. 標準形構築の 1 パス化 (P10 再掲)**
- 対象: `simplex.rs:1231-1273` (行ごと `Vec::new()`)、`:760` (`CsrMat::from_rows` + `CscMat::from_rows` 別々)、
  `presolve.rs:128-169` (`build_a_g` の `Vec<Vec>` → `csr_from_rows`)。nnz を数えて 1 回確保、CSR→CSC は
  カウントソート 1 パス。効果: solve < 3 ms で 1〜2%、fit1d/standgub (構築 5.9%) で 2〜3%。ビット同一。難易度 S。

**C16. ソルバ起動の確保削減 (担当範囲: 初期化)**
- 対象: `extended_dual.rs:2424-2560` (`ColCache::build` ×2 → 1 回 + 差分、`vec![0.0; m]` 群を 1 つの
  `Workspace` に)、`crash` (`:2052`)。効果: solve < 1 ms で 2〜3%。ビット同一。難易度 S〜M。

**C17. `PyModel::solve` 出力の軽量化 (小)**
- 対象: `model.rs:145-155` (`Vec<f64>` → Python list)、`solver.rs:39-45` (目的値の BTreeMap 走査)。
  `x` を `PyList::new_bound` で 1 回確保、目的値は `c·x` を Vec で。効果 < 1% (afiro 2.5 µs = 2%)。難易度 S。

**C18. rayon を presolve から外す** (`redundancy.rs:957-960`)。C1 を採れば自動的に消える。C1 を採らない場合
  でも `par_iter` → 逐次 (`ENOMOTO_PS_NO_RAYON` で実験可)。新プロセス初回の 0.5 ms とスレッド 4 本の
  生成を回避。難易度 S。

**C19. aggregator の候補ソート・fill-in 判定コストの削減** (v2 の本体コスト; 空振りでない 129 回分 152 ms)
- 対象: `aggregator.rs:983-1010` (候補ごとに `a_col_idx[j].sort_unstable(); dedup()`、`coef_of` 二分探索、
  `implied` の再計算)。候補列の索引は構築時に整列済みなので sort/dedup は不要。効果: aggregator 時間の
  1〜2 割、fit2d (11.4 ms) / maros-r7 (5.5 ms) / wood1p (4.5 ms)。ビット同一。難易度 S。

**C20. eqprop の round 数を「変化があった間だけ」に** (現在 2 round 固定、`presolve.rs:549`)。round 0 で
  `forcing_rows == 0 && tightened == 0` なら round 1 を省く (EqPropagateResult に既にカウンタあり)。
  効果: eqprop の半分 (-0.5%)。経路: 2 round 目が何かを見つける問題では同一 (見つけないときだけ省く) →
  ビット同一。難易度 S。

## 4. presolve 各パスの費用対効果 (1 パスずつ切替、全 93 問; 基準 `base3.json`)

| 変種 | 内容 | 幾何平均 | 合計 | 10%超退行 | サイズ/反復が変わった問題 | 目的値不一致 | 10% 以上速くなった問題 (上位) | 遅くなった問題 (下位) |
|---|---|---:|---:|---:|---:|---|---|---|
| noredeq | reduce_equalities を丸ごとスキップ | -9.5% | -4.1% (93 問) | 1 | 2 | なし | beaconfd -53%, standmps -44%, wood1p -44%, lotfi -40%, scagr7 -37%, bore3d -34% | sc50b +6%, fit2d +8%, agg2 +48% |
| noredineq_round | round 末尾の reduce_inequalities をスキップ | -4.7% | -0.9% (93 問) | 1 | 5 | なし | standmps -38%, lotfi -33%, agg3 -31%, scsd6 -20%, standgub -17%, kb2 -16% | greenbeb +5%, fit2d +5%, fit1p +6%, pilot +7%, ganges +8%, maros-r7 +12% |
| nopc | parallelcols 無効 (`ENOMOTO_DISABLE_PARALLELCOLS`) | -3.9% | -0.6% (93 問) | 4 | 29 | なし | standmps -39%, lotfi -32%, agg3 -27%, scsd6 -23%, standgub -21%, afiro -19% | fit2d +9%, fit1p +9%, d2q06c +10%, vtp.base +15%, greenbea +23%, grow7 +50% |
| nodprop | dualpropagate 無効 | -2.1% | -0.2% (93 問) | 3 | 17 | なし | standmps -34%, agg3 -31%, lotfi -31%, scsd6 -21%, greenbeb -17%, fit1p -15% | bore3d +6%, 80bau3b +7%, blend +8%, stocfor2 +16%, boeing2 +16%, etamacro +27% |
| nodbl | doubleton 無効 | +0.3% | +98.1% (93 問) | 7 | 48 | なし | standmps -38%, lotfi -31%, agg3 -30%, scsd6 -17%, czprob -13%, sc105 -11% | fit2d +12%, greenbea +17%, stocfor2 +18%, pilot.ja +21%, pilot +33%, pilot87 +421% |
| noagg | aggregator 無効 (`ENOMOTO_DISABLE_AGGREGATOR`) | +7.7% | +27.0% (93 問) | 32 | 69 | なし | standmps -33%, agg3 -26%, forplan -25%, scsd6 -24%, lotfi -20%, kb2 -16% | sc105 +67%, maros +88%, scorpion +94%, ganges +96%, scrs8 +114%, stocfor2 +211% |
| noeqprop | propagate_equalities (eqprop) 無効 | +10.3% | +18.2% (93 問) | 32 | 69 | なし | agg3 -36%, seba -34%, lotfi -29%, standmps -27%, scsd6 -20%, stocfor1 -13% | greenbeb +89%, fffff800 +103%, cycle +131%, greenbea +146%, pilot +166%, woodw +172% |
| ruiz0 | Ruiz 反復 0 (スケーリングなし) | +2.5% | -6.9% (23 問) | 9 | 21 | なし | agg3 -34%, degen3 -15%, blend -13%, d2q06c -12%, afiro -12%, brandy -12% | bnl1 +21%, agg +23%, bore3d +25%, cycle +27%, boeing1 +28%, capri +30% |

読み方: 「幾何平均」が負なら切った方が速い。「サイズ/反復が変わった問題」が 0 に近いパスは時間だけ
食っている。目的値不一致は全変種でゼロ。ruiz0 は dfl001 が完走せず 23 問で打ち切り (Ruiz 0 は不可)。
standmps/lotfi/agg3/scsd6 の -20〜-40% は基準側のノイズ (全変種で共通に出る)。

パス別の結論:
- **reduce_equalities**: 縮約効果は 4 問 (wood1p/degen2 は残しても反復 ±2%)、コスト 9.6% → 最優先 (C1/C2)。
- **reduce_inequalities(round)**: 効くのは 13 問 (seba/ganges/sierra)、切ると maros-r7 +12% → 切らずに
  境界行を除外 (C5)。
- **parallelcols**: 切ると幾何平均 -3.9% だが 29 問で経路が変わり grow7 +50% → 空振りコストだけ削る (C9)。
- **dualpropagate**: 効くのは 19 問、切ると -2.1% だが etamacro +27% → C11。
- **doubleton / aggregator / eqprop**: 切ると +0.3% / +7.7% / +10.3% (pilot87 +421%, stocfor2 +211%,
  woodw +172%) → 必須。空振り回避 (C3/C10/C20) のみ。
- **scaling**: 0 反復は不可 (退行 9/23 問)。反復数・境界行除外は未計測 (C7)。

## 5. チューニング対象パラメータの棚卸し

| 定数名 | file:line | 現在値 | 役割 | env 上書き | 推奨スイープ値 |
|---|---|---|---|---|---|
| `RUIZ_ITERS` | src/simplex.rs:825 | 10 | Ruiz 等化の反復数 | なし (計装ビルド `ENOMOTO_PS_RUIZ`) | 3, 5, 10, 20 (0 は不可: 退行 9/23) |
| (scaling 零判定) | src/presolve/scaling.rs:153,166,176 | 1e-12 | 列/行ノルムがこれ以下なら更新しない | なし | 1e-14, 1e-12, 1e-10 |
| `RAYON_SIZE_THRESHOLD` | src/presolve/scaling.rs:45 | 100000 | 列ノルム fold の rayon 切替行数 | なし | 実質未使用 (Netlib 最大 12k 行); 撤去候補 |
| `PROPAGATION_PASSES` | src/simplex.rs:829 | 2 | propagate/eqprop/dualpropagate の内部 pass 数 | なし (`ENOMOTO_PS_PROP`) | 1, 2, 4 |
| `PRESOLVE_ROUNDS` | src/simplex.rs:838 | 20 | 外側 round 上限 | なし (`ENOMOTO_PS_ROUNDS`) | 3, 5, 8, 20 |
| `ROWSINGLETON_COLSINGLETON_INNER_ROUNDS` | src/simplex.rs:848 | 1 | 内側 round 上限 | なし (`ENOMOTO_PS_INNER`) | 1, 2, 3 |
| `BIG_M` | src/simplex.rs:860 | 1e7 | フォールバック経路の無限境界代替 | なし | 対象外 (fallback のみ) |
| eqprop の round 数 | src/presolve.rs:549 | `_round_idx < 2` | propagate_equalities を回す round | なし (`ENOMOTO_PS_EQPROP_ROUNDS`) | 1, 2, 4, 全 round |
| 固定点の境界許容 | src/presolve.rs:1092 | 相対 1e-3 | 署名比較で無視する境界変化 | `ENOMOTO_FIXPOINT_EXACT` (=0) | 0, 1e-6, 1e-3, 1e-2 |
| `EPS` (propagate) | src/presolve/propagate.rs:52 | 1e-9 (絶対) | 締め付け採用・不実行可能判定 | なし (相対版 `ENOMOTO_PS_PROP_RELTOL`) | 絶対 1e-9 + 相対 0 / 1e-6 / 1e-3 |
| `doubleton_active` ラッチ | src/presolve.rs:474 | 1 ストライク | 空振り 1 回で以後停止 | なし | 1, 2 ストライク |
| `dualpropagate_active` ラッチ | src/presolve.rs:480 | 1 ストライク | 同上 | なし | 1, 2 |
| `parallelcols` ラッチ | src/presolve.rs:498-499 | 2 連続空振り | 同上 | `ENOMOTO_DISABLE_PARALLELCOLS` | 1, 2, 3 |
| aggregator 有効/種類 | src/presolve.rs:908-919 | v2 | 暗黙自由列の代入消去 | `ENOMOTO_DISABLE_AGGREGATOR`, `ENOMOTO_XROW_AGGREGATOR`, `ENOMOTO_ROWLOCAL_AGGREGATOR` | v2 固定 (無効化 +7.7%) |
| `AggOptions.use_ineq` | src/presolve/aggregator.rs:872 | true | 不等式行由来の暗黙境界を使う | `ENOMOTO_AGG_NOINEQ` | on/off |
| `AggOptions.min_a_count` | :873 | 1 | 候補列の最小等式出現数 | `ENOMOTO_AGG_MINACNT2` | 1, 2 |
| `AggOptions.net_fillin` | :874 | true | 正味 fill-in 判定 | `ENOMOTO_AGG_GROSSFILL` | net 固定 (gross は pilot87 12 倍) |
| `AggOptions.fillin_break` | :875 | true | 連続失敗で打ち切り | `ENOMOTO_AGG_NOBREAK` | on/off |
| `MAX_FILLIN` | src/presolve/aggregator.rs:158 | 10 | 代入 1 本あたりの fill-in 上限 | なし | 5, 10, 20, 50 |
| `MAX_CONSECUTIVE_FILLIN_FAILURES` | :163 | 3 | 打ち切りまでの連続失敗数 | なし | 3, 10, ∞ |
| `SUBSTITUTION_PIVOT_RATIO` (agg) | :152 | 1e-2 | 代入ピボットの行内最大比 | なし | 1e-3, 1e-2, 1e-1 |
| `TOL` (agg) | :149 | 1e-9 | 係数零判定 / 暗黙境界の box 判定 | なし | 1e-12, 1e-9, 1e-7 |
| `TOL` / `SUBSTITUTION_PIVOT_RATIO` (colsingleton) | src/presolve/colsingleton.rs:41,44 | 1e-9 / 1e-2 | 同上 | なし | 同上 |
| `IMPLIED_TOL` | src/presolve/colsingleton.rs:72 | 1e-9 (相対) | 境界保存行を省く含意判定 | `ENOMOTO_KEEP_IMPLIED_BOUND_ROWS` (省かない) | 1e-9, 1e-7, 1e-6 |
| `SUBSTITUTION_PIVOT_RATIO` (freevar) | src/presolve/freevar.rs:106 | 1e-2 | 自由変数代入ピボット | `ENOMOTO_DISABLE_FREEVAR` | 1e-3, 1e-2, 1e-1 |
| `TOL` (doubleton/dualfix/dualpropagate/foldfixed/rowsingleton/parallelcols/freevar) | doubleton.rs:29, dualfix.rs:26, dualpropagate.rs:152, foldfixed.rs:30, rowsingleton.rs:16, parallelcols.rs:114, freevar.rs:91 | 1e-9 | 零判定・費用符号判定・境界一致判定 | なし | 1e-12, 1e-9, 1e-7 (共通化候補) |
| `DENSE_DENSITY_THRESHOLD` | src/presolve/redundancy.rs:111 | 0.03 | 従属等式検出の密/疎切替 | なし (`ENOMOTO_PS_DENSE_THRESHOLD`) | 0 (常密), 0.03, 0.1, 1 (常疎); C1 なら不要 |
| QR rank 許容 | src/presolve/redundancy.rs:272 | 1e-9 (相対) | 密経路の従属判定 | なし | 1e-12, 1e-9, 1e-7 |
| `PIVOT_STABILITY` | src/presolve/redundancy.rs:295 | 0.1 | 疎消去のピボット閾値 | なし | 0.01, 0.1, 0.5 |
| `DEP_TOL` | src/presolve/redundancy.rs:303 | 1e-9 | 疎経路の従属判定 | なし | 1e-12, 1e-9, 1e-7 |
| `MIN_ROWS_FOR_BLOCK_DECOMPOSE` | src/presolve/redundancy.rs:835 | 300 | DM 分解を使う最小行数 | なし (`ENOMOTO_PS_MIN_BLOCK_ROWS`) | 0, 100, 300, ∞ |
| `PARALLEL_DECOMPOSE_ROW_THRESHOLD` | src/presolve/redundancy.rs:811 | 64 | DM 成分を rayon で解く閾値 | なし (`ENOMOTO_PS_NO_RAYON`) | ∞ (逐次) 推奨 |
| reduce_inequalities(round) の実行 | src/presolve.rs:1076 | 毎 round | 重複不等式行除去 | なし (`ENOMOTO_PS_SKIP=reduce_ineq_round`) | 毎 round / 縮約があった round のみ |
| `ineqsingleton` | src/presolve.rs:636 | off | 不等式行の列シングルトン | `ENOMOTO_INEQ_SINGLETON` | on/off (前回: 合計 -0.5%, recipe +40%) |
| `smallcoeff::EPS/CUMULATIVE_FRACTION/NOISE_THRESHOLD` | src/presolve/smallcoeff.rs:104,110,115 | 1e-7 / 0.1 / 1e-10 | 微小係数除去 (未配線) | なし | 配線しない限り対象外 |
| `LARGE` (アロケータ分割) | src/lib.rs:67 | 4 KiB | mimalloc/glibc の振り分け | なし | 1, 4, 16 KiB (16 は degen3 +17%) |

## 6. 優先順位 (期待効果 × 手軽さ)

| 順 | ID | 内容 | 幾何平均の期待効果 | 経路 | 難易度 |
|---|---|---|---|---|---|
| 1 | C1 (+C2, C18) | 従属等式検出を dedupe のみ / 縮約後の小系にだけ | **-7〜-9.5%** (計測済: 丸ごと外して -9.5%, 退行 0, 正答 93/93) | 4 問で縮約結果が変わる | S |
| 2 | C4 | env::var をプロセス 1 回に | -1.5〜-2% (afiro -10%) | 同一 | S |
| 3 | C3 | aggregator v2 の空振り早期 return | -1.5〜-2.5% (fit1d/fit2d -13%) | 同一 | S |
| 4 | C5 (+C14) | reduce_inequalities で境界行を除外 + 容量付き HashMap | -1.5〜-2.5% | 同一 (P7 実装) | S |
| 5 | C6 | 空回り round の打ち切り / 相対しきい値 | -1〜-2% (pilotnov -19%, maros -20%) | 変わる (要 A/B) | S |
| 6 | C7 / C8 | スケーリングの境界行除外 / 平坦ループ | -1〜-2% / -1% | 変わる / 同一 | S |
| 7 | C9, C10, C11, C12, C20 | 各パスの空振りコスト (P3/P4/P9/P11 再提案) | 各 -0.5〜-1.3% | 同一 | S |
| 8 | C13 | 境界を行にしない表現 | -3〜-6% (推定) | 同一 | M〜L |
| 9 | C15, C16, C17, C19 | 構築・起動・出力・aggregator 本体 | 各 -0.5〜-1% | 同一 | S〜M |

C1〜C5 を合わせると (重なりを除いて) 幾何平均で -12〜-15% が見込める。すべて presolve 外の数値経路
(単体法の反復) には触れない。
