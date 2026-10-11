# MIP ソルバの改善策調査 (2026-10-11)

4 つの調査 (LP エンジン / 切除平面 / 分枝限定の探索 / 計算の無駄の監査) を、HiGHS (`highs/mip/*`, `highs/simplex/*`) と
CBC/Cgl のソースと比べて行った結果のまとめ。各項目の採否は `analysis/mip_opt_eval_20261011.md` に記録する
(1 問 90 秒、MIPLIB 40 問、shift 10 秒の幾何平均が 1% 以上良くなれば採用)。

## A. LP エンジン (src/simplex/mip_lp.rs, slope_intercept_dual.rs)

| # | 内容 | 箇所 | HiGHS |
|---|---|---|---|
| L1 | 強分岐の子 LP が反復上限で止まると `objective()` が親の値を返し、利得が常に 0 になる (途中の下界を使うつもりのコードが働いていない) | solver.rs `select_branch_inner` の `_ =>` 分岐、mip_lp.rs `solve_inner` | 反復上限は掛けず、目的値上限と全体予算で制御 |
| L2 | 目的値上限 (打ち切り) の判定が 8 反復ごとの O(n) 走査。増分の双対目的値で毎反復 O(1) に | slope_intercept_dual.rs `ext_check` | `updated_dual_objective_value` / `bailoutOnDualObjective` |
| L3 | 近道 (`fast_finish`) が外れると費用の再摂動つき polish と LU の複製が走る | slope_intercept_dual.rs `polish_with_true_bounds`, `record_duals_with` | `HEkkDual::cleanup` は再摂動しない |
| L4 | `FtLu` の深い複製: 強分岐 1 候補あたり save 1 + restore 2、`basis_inverse_row` でも複製 | mip_lp.rs `save_state`/`restore_state`/`basis_inverse_row` | Playground は基底だけ保存 |
| L5 | 行の活動量・被約費用・基底を 1 ノードで何度も O(nnz)/O(n+m) で作り直す (スラック値を捨てている) | mip_lp.rs `row_activities`/`nb_hint`/`reduced_costs`/`basis` | 解を参照で返す |
| L6 | 求解ごとに ~30 本の作業ベクトルを確保 | slope_intercept_dual.rs `solve_slope_intercept_dual_impl` | HEkk が作業領域を保持 |
| L7 | 強分岐で常に両側の子 LP を解く (片側ずつ評価・再選択にする) | solver.rs `select_branch_inner` | HighsSearch.cpp 391-410 |
| L8 | 特異な復元基底で cold start に落ちる (`factorize_basis_repair` を warm start で使っていない) | slope_intercept_dual.rs `warm_start_basis` | HFactor の rank 不足処理 |
| L9 | `set_basis` が基底の順序を並べ直し、同じ集合でも LU/DSE のキャッシュを捨てる | mip_lp.rs `set_basis` | 同じ基底なら INVERT を保つ |
| L10 | 伝播で締まった連続列の境界まで LP に流す | solver.rs `sync_lp` | `removeContinuousChangedCols` |
| L11 | 行の追加・削除ごとに標準形・PRICE 行列を全部作り直す | mip_lp.rs `rebuild` | `addRows` 増分 |
| L12 | 復元基底の DSE 重みがないとき単位重み、polish でずれた重みを保存 | mip_lp.rs `dse_cache` | Devex / 厳密計算 |
| L13 | warm start の被約費用を毎回 BTRAN + PRICE で作り直す | mip_lp.rs `set_warm_d` | 同じ (computeDual) |
| L14 | 反復上限の救済が全スラック基底から無制限に解き直す | solver.rs ノード LP | IPM で基底を作り直す |
| L15 | `restore_state` が全列を比較・複製 | mip_lp.rs | — |
| L16 | 内点法の監視スレッドを毎回 spawn、`solve_race` が rayon プールを毎回作る | mip_lp.rs | — |

## B. 切除平面 (src/mip/cuts.rs, separation.rs, zerohalf.rs, clique.rs)

| # | 内容 | HiGHS / Cgl |
|---|---|---|
| C1 | カットプールにエージング・削除がなく、根の全候補を入れて毎ノード全走査 | `HighsCutPool::separate` (agelim, softlimit) |
| C2 | ベース不等式の長さ制限・境界上の項の除去がない。tableau 行の BTRAN が密 | `preprocessBaseInequality`, `getBasisInverseRowSparse` |
| C3 | 分離が逐次 (rayon 未使用) | — |
| C4 | 整数台のカットを整数係数にスケールして右辺を切り下げていない | `postprocessCut` (`integralScale`) |
| C5 | 小さい係数の除去閾値 1e-9·cmax、dynamism > 1e6 でカットごと破棄 | 1e-4·cmax を境界で吸収 |
| C6 | CMIR の δ 候補とノルムの取り方 | `cmirCutGenerationHeuristic` |
| C7 | 根で外したカットをプールから戻す経路が既定で無い | `separationRound` の `cutpool.separate` |
| C8 | 変数上下限 (VUB/VLB) を 2 項行からしか作らず、プロービングの含意・暗黙境界カットがない | `HighsImplications` |
| C9 | zerohalf が元の整数行だけ・CG 丸めのみ | `HighsModkSeparator` (mod-k + CMIR) |
| C10 | 削除と追加を別々に行い LP を 2 回作り直す | `addCuts/removeCuts` 増分 |
| C11 | 最適面の分離で LP を毎ラウンド 3 回 clone | — |
| C12 | カット生成の作業領域 (HashMap 等) を毎回確保、`lp.row(i)` の clone | `HighsTransformedLp` |
| C13 | 2 種の経路集約が重複候補を作る | `HighsPathSeparator` 1 本 |
| C14 | `age_cuts` が基底・活動量を毎ノード複製 | `performAging` は参照 |
| C15 | tableau 行の選択に DSE 重みを使っていない (`dse_weight` が 1.0 固定) | DSE 重み + `‖w‖²` |
| C16 | lifted cover と CMIR の採否基準 | `tryGenerateCut` |
| C17 | 右辺の累積が素の f64 | `HighsCDouble` |
| C18 | clique グラフを辺集合で持つ | `HighsCliqueTable` |

## C. 分枝限定の探索 (src/mip/solver.rs, queue.rs, pseudocost.rs, domain.rs, heuristics.rs)

| # | 内容 | HiGHS |
|---|---|---|
| B1 | 強分岐の予算・信頼度しきい値 (1..5)・反復上限 (10..500) が小さい | `sbmaxiters = 1e5 + (LP 反復)/2`、minreliable 8 |
| B2 | 片側ずつの強分岐、子 LP 解で他候補のスコア更新、ノード内の再評価回避 | `analyzeSolution`, `reliableatnode` |
| B3 | プロービングの含意 (`conflict_edges`) を木の伝播に使っていない | `cliquetable.addImplications` |
| B4 | 強分岐で候補ごとに LP 状態 (LU 込み) を保存 | Playground |
| B5 | 再スタートで擬似費用を捨てる | `HighsPseudocostInitialization` |
| B6 | 木の途中の再スタートがない | `checkRestart` |
| B7 | RINS が一致率 50% 未満で何もしない、木の中の LNS の呼び方が固定的 | RINS の近傍拡大、`moreHeuristicsAllowed` |
| B8 | 根の被約費用固定が最後の LP 1 回分だけ | `addRootRedcost` (各ラウンド) |
| B9 | 目的関数の伝播が既定で無効、LP 前に目的の最小活動量で刈らない | `objProp_` |
| B10 | ノードあたりの O(n+m) の複製・走査 | 参照アクセス、`integral_cols` |
| B11 | ノード LP からの擬似費用観測の delta が強分岐と単位が違う | 分枝距離のみ |
| B12 | 潜る側の選択 (根の LP 解の方向)、潜りの打ち切り規則 | kRootSol, `backtrackPlunge` |
| B13 | 暗黙の整数列を伝播・分枝で整数扱いしない | `implint_cols` |
| B14 | 局所被約費用固定を連続列にも適用、`moved` の全列走査 | 整数列のみ |
| B15 | `try_lp_solution` が改善しない実行可能解でも LP を解き直す | — |
| B16 | リテラル衝突プールが上限で FIFO 上書き | 年齢で消す |

## D. 計算の無駄 (監査)

| # | 内容 |
|---|---|
| W1 | `basis()` がノードごとに 4〜5 回 O(n+m) ×3 本を確保 (使うのは「基底か否か」だけ) |
| W2 | `age_cuts` が毎ノード全行の活動量 O(nnz) |
| W3 | `solve_inner` の求解ごとの O(n+m+nnz) の付帯処理 (`nb_hint` の活動量、`const_obj`、clone) |
| W4 | 強分岐の save/restore が LU を含む全状態を clone (= L4/B4) |
| W5 | カット行の追加・削除で標準形を丸ごと作り直す (= L11/C10) |
| W6 | `reduced_costs()` と `basis()` をノードごとに 2 組 |
| W7 | ノードごとの `zi_round`/`simple_rounding` の O(nnz) |
| W8 | 診断用 `dbg_lost` の `format!` がデバッグ解なしでも毎ノード走る |
| W9 | 兄弟ノード・待ち行列からの取り出しで根まで巻き戻して積み直す |
| W10 | 衝突解析ごとに O(n) の表を作り直す、`add_proof_from` の行 clone |
| W11 | 分枝時に基底・保護カット・DSE を 3 回走査 |
| W12 | `MipLp::row()` が行を clone (`separate` は全 LP 行を clone) |
| W13 | `NodeQueue.protect` が HashMap (連番なので Vec でよい) |
| W14 | `nnz`・整数列数を毎回数え直す |
| W15 | `ensure_col_rows` が `p.cols` の複製を作る |
| W16 | `pool_cut_round` の全ソート (select_nth で足りる) |
| W17 | `apply_dual_proofs` のノードごとの小確保 |
| W18 | `separate()` 呼び出しごとの O(nnz) 前処理 |
| W19 | `objective()` が O(n) でノードごとに数回 |
| W20 | `try_lp_solution` の二重 clone |
| W21 | ノード基底の復元・DSE 当てはめの O(n+m) 確保 |
| W22 | `Domain::propagate` の二重計算と TLS アクセス |
| W23 | `changes_since` が Vec を返す |
| W24 | 被約費用固定後の全列走査 |
| W25 | `xprof` の無効時コスト (thread_local borrow) |
| W26 | `NodeQueue::prune` の戻り値が未使用で毎回確保 |
| W27 | `orb_col`/`orb_others` の HashMap・HashSet |
| W28 | `remove_cuts_older_than` の毎回の確保 |
| W29 | `age_cuts` の `nslack` 確保と `row_bounds` 二重呼び |
| W30 | LP 解・行値を参照で返す API がない (W3, W6, W19, W20 の土台) |

`env_str!`/`tunable!` は呼び出し箇所ごとの `OnceLock` にキャッシュされており、ホットループでの getenv はない。
