# NETLIB93 全体の非効率箇所の計測と対処法一覧 (2026-09-23)

基点: ブランチ `claude/practical-tesla-s52fnq` HEAD `3d51b78` (= /tmp/claude-0/venv_base のビルド、src は同一)。
コードは変更していない。直前の監査 `waste_audit_20260923_000000.md` で実施済みの対策
(P1〜P8, L1〜L5, D1〜D6, U1〜U4) は再提案しない。同ファイル末尾「未実装の候補」のうち
本計測で裏付けが取れたものは、計測値を付けて再掲している (その旨を明記)。

## 0. 計測方法と生データ

- 4 コア、`/tmp/claude-0/venv_base/bin/python`、Netlib 93 問 (`.netlib_cache/mps`)。
- **全 93 問一括計測 (1 回)**: 問題ごとに新プロセス、`_build_our_model` → `model.solve()` を
  約 0.3s 分 (最大 10 回) 反復し中央値。`ENOMOTO_PROF_PHASES_EXT=1 ENOMOTO_PROF_PRESOLVE=1
  RAYON_NUM_THREADS=1` を付けた (プロファイル出力の eprintln 分だけ小問題の presolve 合計は
  膨らむ。各ステップの個別計時は eprintln を含まない)。
  生データ: `analysis/overall_inefficiency_20260923_064827_sweep.json` (問題ごとの build/solve/
  presolve/主ループ wall/反復数/フェーズ内訳/presolve ステップ内訳)。
- **反復数の追加計測**: `ENOMOTO_DEBUG_EXT_ITERS=1` で全 93 問 (polish 反復・cleanup ピボット・
  primal handoff の有無)。`..._iters.json`。
- **命令数プロファイル**: 記号付き release ビルド (`CARGO_PROFILE_RELEASE_DEBUG=1`、コードは同一)
  を scratchpad に作り `valgrind --tool=callgrind --toggle-collect='*PyModel*solve*'` で
  kb2 (×3), ganges, bnl1, pilot, pilot87 を採取 (RAYON_NUM_THREADS=1)。
- **初回呼び出しコスト**: 同一プロセス内で 1 回目と 2 回目以降の `solve()` を比較、`strace` で
  1 回目のスレッド生成・mmap を確認。
- perf は無いので使っていない。

### 0.1 全体像 (一括計測、プロファイル ON)

| 群 | solve 合計 | presolve | 主ループ (拡張双対 wall) | その他 (std_form 構築・finish/polish・FFI) |
|---|---:|---:|---:|---:|
| 93 問 | 36.2 s | 2.18 s (6.0%) | 33.5 s (92.4%) | 0.58 s (1.6%) |
| 重い 12 問 (≥250ms) | 32.9 s | 1.13 s (3.4%) | 31.4 s (95%) | 0.40 s |
| 軽い 81 問 (<250ms) | 3.31 s | 1.05 s (32%; eprintln 込み。ステップ計時の合計は 0.85 s = 26%) | 2.08 s (63%) | 0.18 s (5%) |

主ループのフェーズ合計 (ms、全体 / 重い 10 問 / 軽い 83 問):
ftran (入列 + DSE τ の融合 FTRAN) 8077 / 7441 / 486、**refactor 5757 / 5449 / 223**、bfrt 3583 / 3365 / 170、
price 3148 / 2868 / 241、btran 3113 / 2870 / 191、xb_update 2807 / 2631 / 136、ft_update 1460 / 1297 / 134、
chuzc1 1160 / 1000 / 148、chuzr 633 / 571 / 52、dse_update 462 / 406 / 47、dual_update 359 / 330 / 27。

presolve ステップ合計 (ms、全体 / 軽い 83 問): reduce_equalities 496 / 217、**aggregator 261 / 122**、
reduce_inequalities(round) 151 / 69、propagate 146 / 47、doubleton 139 / 76、parallelcols 127 / 72、
scaling::compute 102 / 56、dualpropagate 59 / 23、colsingleton 58 / 28、scaling::apply 51 / 27、
rebuild_g(inner) 44 / 16、eqprop 42 / 23、extract_bounds(inner) 30 / 13、reduce_inequalities 29 / 15、
foldfixed(G) 28 / 9、dualfix 22 / 9、foldfixed(A) 20 / 11、final propagate 18 / 9、rowsingleton 11 / 5。
外側ラウンド数: 2 回 3 問、3 回 25、4 回 34、5 回 10、6〜18 回 15、上限 20 回 6 問。

### 0.2 callgrind (命令数) の要点

| 問題 | presolve | 主ループ | 主ループ内の上位 |
|---|---:|---:|---|
| kb2 (41×43) | 66% | 29% | malloc/free/realloc が全体の約 30% |
| ganges | 53% | 45% | malloc 系 25%、pair FTRAN 13%、commit_update 4.4%、btran 4.3% |
| bnl1 | 17% | 82% | pair FTRAN 22.9% (うち `pair_r_u_permute` 17.4%)、btran_tail 6.8%、refresh_row 6.4%、refactorize 6.2%、commit_update 5.6% (pack_scaled_dense 4.0%)、compute_rhs_affine+residual_norm_affine 5.0%、memset 3.3% |
| pilot | 7% | 92% | pair FTRAN 29.3% (U 段の `dense[i] += alpha*v` だけで 14.7%)、refactorize 14.8% (eliminate 6.4%、find_best_pivot 3.9%)、btran_tail 7.9%、refresh_row 4.4%、Vec 伸長 (grow_one/finish_grow/realloc/memcpy) 約 5% |
| pilot87 | 2.5% | 97% | pair FTRAN 26.3% (U 段 axpy 14.9%)、**refactorize 25.8%** (eliminate 14.0%、find_best_pivot 8.8%、`KernelMatrix::row_get` 5.1%)、btran_tail 8.0%、**finish→polish→primal `run_phase` 4.3%** (216 反復、毎反復 resync)、refresh_row 3.4%、commit_update 2.5% |

### 0.3 その他の計測値

- 融合 FTRAN の内訳 (`ENOMOTO_FUSED_DSE_FTRAN=0` で分離): pilot 入列 55 µs/iter + DSE τ 64 µs/iter
  (融合時 108)、25fv47 12.6 + 16.0 (融合 27.2)。**DSE の τ (密) は入列 FTRAN より高い**。
- 再分解: pilot87 63 回 × 30.7 ms (wall の 30%)、うち drift トリガ 42 / verify 8 / bump 10 / clock 3。
  ピボット探索 600 ms、col_max 再走査 12.7M 要素。dfl001 90 回 × 24.5 ms (14%)、drift 23 / clock 67、
  探索 1228 ms、探索上限到達 32.5%、候補 131/step。pilot 30 回 (drift 22)、greenbea 30 回 (drift 23)、
  grow22 27 回 (drift 22)、25fv47 29 回 (drift 17)。
- 初回呼び出し: 新プロセスでの 1 回目は 2 回目以降より **0.6〜1.0 ms** 遅い (afiro 1.09 vs 0.39、
  kb2 1.67 vs 0.85、sc50a 1.39 vs 0.67、adlittle 2.52 vs 1.66、e226 9.59 vs 8.65)。strace: 1 回目だけ
  `clone3` ×4 (rayon グローバルプール生成) と各スレッドの 64〜128 MB arena mmap。`benchmark_highs`
  は 1 プロセス 1 問なので **ベンチの `ours_time` には毎回この初回コストが乗る**。
- 密経路 `drop_linearly_dependent` (faer ColPivQr) を通る問題: 24 問 (adlittle, afiro, agg, beaconfd,
  blend, boeing1, boeing2, brandy, e226, fit1d, fit2d, forplan, grow7, kb2, lotfi, sc50a, sc50b, scagr7,
  scsd1, share1b, share2b, stocfor1, vtp.base, wood1p)。reduce_equalities の壁時計: kb2 初回 0.66 ms /
  2 回目以降 0.06〜0.2 ms (17×16 の QR に対して過大)、`RAYON_NUM_THREADS=1` だと 1〜3.8 ms に悪化
  (プールへの受け渡し待ち)、beaconfd 1.7〜2.9 ms、**wood1p 75 ms (総時間 193 ms の 40%)** かつ
  実行ごとに変動 (既知の非決定性の源)。
- polish: 93 問すべて polish 反復 0、cleanup ピボット 0。53/93 問が polish 直後に primal `run_phase`
  へ handoff。pilot87 ではそこで 216 反復 (4.3%)、kb2 でも 3.2%。
- ベンチハーネス `_build_our_model` (Python) は `ours_time` 外だが全体で 34.9 s (solve 合計と同じ
  だけ): dfl001 5.5 s、fit2p 5.7 s、fit2d 3.2 s、80bau3b 3.0 s、maros-r7 2.9 s (highspy の属性を要素ごとに
  読むため二乗オーダー)。

## 1. 対処法一覧

各項目: 対象 (file:line)、非効率の内容と計測値、対処法、効果見積もり (10% 以上速くなりそうな問題)、
経路への影響とリスク、難易度 (S/M/L)。「推定」と書いた数値は計測から外挿した推測。

### P. presolve と小問題の固定費

**P1. 従属等式検出の密経路が faer の並列 QR を呼び、rayon プール生成・受け渡しが支配的**
- 対象: `src/presolve/redundancy.rs:198-240` (`drop_linearly_dependent`: `ColPivQr::new` をグローバル
  並列度 = Rayon で実行)、呼び分け `redundancy.rs:132-160` (`DENSE_DENSITY_THRESHOLD = 0.03`)。
- 計測: 上記 0.3。kb2/sc50b/afiro/vtp.base/beaconfd など密経路 24 問で reduce_equalities が presolve
  最大項目 (kb2 1.7 ms、beaconfd 2.1 ms、vtp.base 1.4 ms; 一括計測は RAYON=1 で膨らんだ値だが、
  既定でも初回 0.6 ms + 2 回目以降 0.1〜0.2 ms)。新プロセスでは rayon の 4 スレッド生成 (0.5〜0.7 ms)
  がこの経路で初めて起きる。wood1p は 2595×243 の密 QR に 75 ms。
- 対処: (a) `faer::set_global_parallelism(Parallelism::None)` を 1 回呼ぶ (または `qr_in_place` に
  `Parallelism::None` を明示) — この crate で faer の並列が意味を持つ規模はない。これで初回の
  スレッド生成も消え、wood1p の非決定性も消える。(b) `p ≤ 数十` の小行列は自前の逐次 Householder
  (n+1)×p で十分。(c) wood1p 型 (p=243, n=2595) は密経路の O(p²n) が高い: 既存の疎経路
  `drop_linearly_dependent_sparse_blocked` に回す閾値 (行数 p ≥ 100 なら疎) を検討 — こちらは落とす
  従属行が変わり得るので経路変更扱い。
- 効果: (a) だけで密経路 24 問のうち solve < 10 ms の約 15 問で **10〜40%** (kb2, sc50a, sc50b, afiro,
  adlittle, vtp.base, beaconfd, blend, share2b, stocfor1, scagr7, boeing2, e226, brandy, lotfi)、ベンチの
  単発計測ではさらに初回スレッド生成分 (0.5 ms) が全問題から消える (solve < 5 ms の約 25 問で 10〜50%)。
  (c) は wood1p -30% 程度 (推定)。
- 経路: (a)(b) は QR の結果が逐次実行の値になる。faer の並列/逐次で丸めが変わる可能性はあるが、
  落とす行の判定は 1e-9 の相対閾値なので実用上同一。(c) は縮約結果が変わり得る。
- 難易度: (a) S、(b) S、(c) M。

**P2. aggregator が毎ラウンド無条件に A/実行行/b/c を丸ごとコピーし、何も消せなくても A を再構築**
- 対象: `src/presolve/aggregator.rs:291-300` (`csr_rows(a)`, `real_rows.to_vec()`, `b.to_vec()`,
  `c.to_vec()`, 列→行索引 `Vec<Vec<usize>>` 構築)、`:493-500` (`csr_from_rows(&final_a_rows)` を
  substitutions が空でも実行)。呼び出し側 `src/presolve.rs:868-893` はラッチなし (毎ラウンド)。
- 計測: 全体 261 ms、軽い 83 問で 122 ms (軽い群の solve の 3.7%)。agg 0.88 ms/11.9 (7%)、scrs8 2.4/26
  (9%)、bandm 1.4/18 (8%)、tuff 1.6/16 (10%)、scorpion 0.44/5.0 (9%)、ganges 2.45/33.5 (7%)。
  callgrind kb2 で 8%、ganges 7.2%。
- 対処: 候補探索 (`row_implies_own_bound`) を CSR スライス上で先に行い、候補が空なら
  `AggregatorResult { a: a.clone(), ... }` で即返す (colsingleton.rs:216 と同じ流儀)。候補があるときだけ
  行リストへ展開する。さらに doubleton/dualpropagate と同様の 1 ストライクラッチ (空だったら以後
  呼ばない) は経路 (縮約結果) を変え得るので別 A/B。
- 効果: 軽い 83 問で平均 2〜4%、上記 6 問は 5〜10%。ビット同一 (早期 return は canonical な A の
  clone; `csr_from_rows` が canonical 入力に対して同一物を作ることは P3 で既に前提)。
- 難易度: S。

**P3. doubleton が何も見つからなくても A/G 全行を rewrite_row で書き直して CSR を再構築**
- 対象: `src/presolve/doubleton.rs:89-200` (`csr_rows_pruned(a)`、`extract_bounds`、全行の
  `rewrite_row`、`csr_from_rows` ×2 — `subs.is_empty()` の早期 return が無い)。
- 計測: 全体 139 ms、軽い群 76 ms。seba 0.77/10.3 (7%)、standata 0.57/9.0 (6%)、bore3d 0.26/4.5 (6%)、
  beaconfd 0.27/5.9、tuff 0.87/16 (5%)。ラッチにより各問題で必ず 1 回は「空振りの全再構築」を払う。
- 対処: 候補走査を CSR 行スライス (`col_indices_of_row_raw(i).len() == 2`) で行い、`subs` が空なら
  `(a.clone(), b.to_vec(), g.clone(), h.to_vec(), c.to_vec())` を返す。空でない場合も `csr_rows_pruned`
  は不要 (行スライスを直接 `rewrite_row` に渡せる)。
- 効果: 軽い群 1〜3%、上記 5 問で 5〜7%。ビット同一。
- 難易度: S。

**P4. parallelcols が毎ラウンド CSR→Vec<Vec> コピーと CSC 構築をやり直す**
- 対象: `src/presolve/parallelcols.rs:169-201` (`csr_rows(a)`、`CscMat::from_entry_stream`)。
- 計測: 全体 127 ms、軽い群 72 ms。standata 0.73/9.0 (8%)、standgub 同程度、fit2d 10.1/143 (7%)、
  israel 0.12/4.8、bnl1 0.39+0.29+0.26 (3 ラウンド分 = 1.5%)。
- 対処: 列署名は行を CSR スライスから直接読んで作れる (CSC を作らず `a.as_ref()` と `real_rows` を
  列ごとに走査する転置 1 パス、または `CscMat::from_rows` の 1 回構築で `csr_rows` を省く)。
  A が前ラウンドから変わっていない (`a.nrows()`/nnz 同一かつ他パスが何も消していない) 場合は前回の
  署名テーブルを再利用。
- 効果: 軽い群 1〜3%、standata/standgub/fit2d で 5〜8%。ビット同一。難易度: S〜M。

**P5. 外側ラウンドの固定点判定が「まる 1 ラウンド無駄に回してから」しか止まれない**
- 対象: `src/presolve.rs:1032-1037` (署名比較はラウンド末尾)、`:507-520`。
- 計測: 最終ラウンドのステップ合計 126 ms (= presolve ステップ合計の 7%)、軽い群 73 ms。fit1d 11%、
  fit2d 10%、agg2 7%、agg3 6%、czprob 5%、standata/standgub 5% (各問題の solve 比)。
- 対処 (ビット同一): ラウンド先頭の `propagate` 出力 (lb/ub/real_rows 数/a.nrows) が前ラウンドの
  propagate 出力と一致し、かつ前ラウンドの propagate 以降の全パスが「何も変えなかった」フラグを
  立てていれば、残りのパスは前ラウンドと同じ入力に対する同じ決定的関数なので結果も同じ → その場で
  break。各パスの「変えなかった」は既に戻り値 (substitutions/fixes/implied の空、行数一致) で分かる。
- 効果: 最終ラウンドの約 7 割を省き、軽い群 1〜2%、fit1d/fit2d/agg2/agg3 5〜8%。難易度: S。

**P6. presolve 全体の Vec<Vec<(usize,f64)>> ↔ faer Csr 往復による malloc 支配**
- 対象: `src/presolve.rs` の各パス境界 (`csr_rows(&a)` :606/:621、`csr_from_rows` :611、各パス内部の
  `csr_rows`/`csr_from_rows`: aggregator 6 箇所、doubleton 9、dualpropagate 10、parallelcols 13、
  propagate 27、colsingleton 7 …)。1 ラウンドで A は 5 回以上、G は 4 回以上作り直される。
- 計測: callgrind で kb2 の命令数の約 30%、ganges の約 25% が malloc/free/realloc (`_int_malloc`
  6.7%、`_int_free` 4%、`finish_grow` 5%、`grow_one` 5%、`SpecFromIterNested` 6%、memset 4%)。
  presolve は軽い群の solve の 26〜32%。
- 対処: (a) ラウンド内で A・実行行を「所有する行リスト (Vec<Vec>)」のまま持ち回り、Csr が必要な
  消費者 (`propagate`/`extract_bounds`/`reduce_inequalities`/`dualpropagate` の `as_ref()` 読み) に
  だけ `CsrRowBuilder` で 1 回構築する (先の監査 P3 の `csr_from_rows_direct` を「作らない」方向に
  進める)。(b) 行 Vec の再利用: `rewrite_row` / `fold_fixed_columns` / `propagate` の
  `kept_rows` は容量を持ったバッファを取り回す。(c) 全パス共通の `SparseAccum` を 1 個にする。
- 効果: 軽い群 5〜10% (推定: presolve の malloc 分の半減)。ビット同一 (順序・値は同じ)。
- 難易度: M〜L (影響範囲が広い)。

**P7. `reduce_inequalities(round)` が毎ラウンド、境界行 (単項行 n 本) までハッシュする**
- 対象: `src/presolve/redundancy.rs:1289-1360`、呼び出し `presolve.rs:1029`。
- 計測: 全体 151 ms、軽い群 69 ms (scrs8 1.5/26 = 6%、bandm 1.0/18、agg 0.63/11.9 = 5%)。G の行の
  大半は `rebuild_g_ref` が作る単項の境界行で、変数ごとに (j,+1)/(j,-1) 高々 1 本ずつ = 同士では重複
  し得ない。
- 対処: 長さ 1 の行はハッシュ・比較対象から外す (kept にそのまま入れる)。実行行が置換で単項に
  潰れた行が境界行と重なるケースは、次ラウンド先頭の `extract_bounds` が両方を lb/ub に畳むので
  最終結果は同じ; ただし G の行数が 1 ラウンドだけ違うため署名固定点のタイミングが 1 ラウンド
  ずれる可能性がある (縮約結果は同一のはずだが要 `ENOMOTO_DEBUG_PRESOLVE_HASH` で確認)。
- 効果: 軽い群 1〜2%。難易度: S。

**P8. `scaling::compute` が 10 反復すべてで faer の行イテレータ経由、`apply` が行ごとに Vec 生成**
- 対象: `src/presolve/scaling.rs:119-209`。
- 計測: compute 102 ms / apply 51 ms (軽い群 56 / 27)。fit2d 9.0/143 (6%)、seba 0.62/10 (6%)、bore3d
  0.23/4.5 (5%)、standata 0.32/9.0、israel 0.15/4.8。
- 対処: `col_indices()`/`values()` の平坦スライスと `row_ptr` で直接ループ (2 行列を 1 本に連結)、
  `col_norm`/`row_norm` の更新を 1 パスに融合、`apply` は `CsrRowBuilder` に直接書く。反復回数の
  削減 (10→5) は結果が変わるので別扱い。
- 効果: 軽い群 1〜3%。ビット同一 (演算順序を変えないこと)。難易度: S。

**P9. 内側ループの `rebuild_g(inner)` → 直後の `extract_bounds(inner)` 往復**
- 対象: `src/presolve.rs:735-740` と `:790-802`。rowsingleton が何も固定せず doubleton/colsingleton も
  空のパスでは、G を境界行込みで組み直してから同じものを lb/ub/実行行に分解し直している。
- 計測: rebuild_g(inner) 44 ms + extract_bounds(inner) 30 ms (軽い群 29 ms、約 1%)。
- 対処: `rs.fixes` が空かつ両置換が空なら g/h と lb/ub/実行行は不変なので両方スキップ (ビット同一)。
- 効果: ~1%。難易度: S。

**P10. `build_std_form_presolved` の行組み立てと ColCache/PriceMatrix の構築 (小問題の固定費)**
- 対象: `src/simplex.rs:1200-1290` (`rows: Vec<Vec>` に push → `freeze_std_matrices`)、
  `src/simplex/extended_dual.rs:2059-2071` (`ColCache::build`: `Option<Affine1>` ×3 本を個別 collect)、
  `:2195-2245` (PRICE 行列を一度作ってから非基底/基底で並べ替え)。
- 計測: kb2 callgrind で `build_std_form_presolved` から `run_extended` を除いた分 4.8%、
  `solve_lp_dual_extended` のループ外 (ColCache/PRICE 行列/crash/初回分解/seed FTRAN) が数%。
  一括計測の「その他」は solve < 10 ms の問題で 10〜29% (sc50a 24%、scfxm1 24%、adlittle 29%、
  sc205 21%) — ただしここには finish/polish/primal handoff (L13/L14) も入る。
- 対処: 行リストを `CsrMat` に直接書く (nnz を先に数えて 1 回確保)、ColCache を 1 パスで SoA
  (base/slope/has フラグ) に、PRICE 行列は最初から非基底/基底に分けて書く。
- 効果: solve < 10 ms の約 30 問で 2〜5%。ビット同一。難易度: S。

**P11. dualpropagate が有効な間、毎ラウンド A+実行行の転置を作り直す**
- 対象: `src/presolve/dualpropagate.rs:200-300` (前監査「未実装」の再掲)。
- 計測: 59 ms (軽い群 23 ms、maros-r7 16 ms = 1.2%)。ラッチが効くので多くは 1〜2 ラウンド。
- 対処: A が変わらなかったラウンドは前回の転置を再利用。効果 <1%。難易度: S。優先度低。

### L. 主ループ (拡張双対) / LU

**L1. 融合 FTRAN の U 段が 2 本のベクトルを別々に axpy している**
- 対象: `src/simplex/lu.rs:4101-4140` (`pair_r_u_permute`: `if scratch_a[p] != 0 { axpy } if
  scratch_b[p] != 0 { axpy }`)。L 段 (`l_solve_into_pair` :2836-2866) は既に `(true,true)` を 1 ループで
  処理しているのに U 段は未対応。
- 計測: U 段の `dense[i] += alpha * v` が pilot 14.7%、pilot87 14.9%、bnl1 4.8% (命令数)。入列 α_q の
  密度は 0.43〜0.999 (bnl1 0.43、dfl001 0.64、pilot87 0.83、fit2p 0.999)、τ はほぼ密なので、eta の
  6〜8 割で両方が非ゼロ = 同じ eta の index/value を 2 回ロードしている。
- 対処: `(xa != 0, xb != 0)` で 4 分岐し、両方非ゼロなら `for &(i,v) in eta { a[i] += xa*v; b[i] += xb*v }`
  (HybridVec に `axpy2_into_dense` を追加; dense アームも zip で 1 パス)。各要素の演算は同一なので
  ビット同一。
- 効果: U 段の索引/値ロードが 2〜3 割減 → pilot/pilot87/dfl001/d2q06c/25fv47 で **3〜5%** (推定)、
  fit2p (密) でも 3%。難易度: S。

**L2. Markowitz ピボット探索が候補列の各要素で行側二分探索 (`row_get`) を引く**
- 対象: `src/simplex/lu.rs:1108-1170` (`find_best_pivot`: `self.mat.row_get(i, j)` を候補ごと/
  `col_max_abs_rescan` :1049-1058 でも行ごとに `row_get`)。列ミラー `KernelMatrix::col` は添字のみ
  (u32) で値を持たない。
- 計測: pilot87 探索 600 ms = refactor 1932 ms の 31% = **総時間の 9%**、col_max 再走査 12.7M 要素、
  `KernelMatrix::row_get` 単独で 5.1% (命令数)。dfl001 探索 1228 ms = refactor の 56% = **総時間の 8%**
  (候補 131/step、32.5% が上限 256 到達)。d2q06c 105 ms (5%)。
- 対処 (ビット同一): 列ミラーに値も持たせる (`col_val: Vec<f64>`、`col_insert`/`eliminate` の行書き換え
  時に同じ位置を更新 — `eliminate` は行をマージし直すので、影響行の各要素について列ミラー側の値を
  書き戻す 1 パスを追加)。これで `min_pivot` 判定と col_max 再走査から二分探索が消える。候補評価
  順序・閾値・タイブレークは不変。
- 効果: pilot87 5〜7%、dfl001 4〜6%、d2q06c/pilot/maros-r7 2〜3% (推定: 探索時間の 6〜7 割が row_get)。
- 難易度: M (eliminate 内の列ミラー更新が要点)。

**L3. x_B ドリフト検査による再分解が CLOCK より早く起き、再分解回数を 2〜3 倍にしている (経路変更)**
- 対象: `src/simplex/extended_dual.rs:4200-4245` (`XB_CHECK_INTERVAL=5` ごとに residual、
  `XB_DRIFT_TOL = 1e-8` :646、10 回ごとに 10 倍緩和 :656-669)。
- 計測: 再分解の原因 — pilot87 drift 42/63、pilot 22/30、greenbea 23/30、grow22 22/27、d2q06c 22/42、
  25fv47 17/29、maros-r7 20/28、dfl001 23/90。再分解 1 回 = pilot87 30.7 ms、pilot 8.8 ms。
  CLOCK だけなら pilot87 は約 300〜450 更新 (max_update_streak 289) ごと。
- 対処: `XB_DRIFT_TOL` を 1e-7〜1e-6 に、または相対残差 (`||b||` スケール) に切り替えて A/B。HiGHS は
  この検査自体を持たない (更新数 + 合成クロック + 数値異常のみ)。
- 効果: pilot87 最大 -15%、pilot/greenbea/grow22 -5〜10% (推定; 再分解が半減した場合)。
- 経路: **変わる** (再分解位置が変わると x_B の丸めが変わる)。数値的に危険な問題 (pilot.ja illcond 6
  回) で反復が増えるリスク。難易度: S (パラメータ) だが A/B 必須。

**L4. BFRT の combined-flip FTRAN を入列/τ の融合 FTRAN に 3 本目として融合 + flip 反映と x_B 更新の統合**
- 対象: `src/simplex/extended_dual.rs:3520-3570` (`lu.solve_into(&combined_base, …)`) と
  `:3573-3600` (`for i in 0..m` の反映ループ ×1〜2 + touched リセット)、`:3912-3935` (xb_update)。
  前監査「未実装: flip 用と entering 用の x_B 更新の統合 (fit2p 推定 -7%)」「BFRT FTRAN の融合」の再掲。
- 計測: fit2p bfrt 24% + xb_update 13% (flip 3.4 本/iter、combined 密度 0.49、α_q 密度 0.999)、fit1p
  bfrt 21% + xb 15%、dfl001 bfrt 11% + xb 11%、pilot87 9% + 3%。
- 対処: (a) combined-flip は入列選択後・入列 FTRAN 前に確定するので、`solve_into_pair_capture` を
  3 本版にして L/R/U を 1 走査で解く (順序を保てばビット同一)。(b) flip 反映の `for i in 0..m` と
  xb_update のループを 1 本にし `refresh_row` を行ごと 1 回にする — 行の更新順が変わるため
  `InfeasibleRows` の内部順序 → CHUZR のタイブレークが変わり得る (経路変更の可能性)。
- 効果: (a) fit2p/fit1p 5〜8%、dfl001 3%; (b) fit2p さらに 5% (前監査の推定)。難易度: M。

**L5. FT 更新 `commit_update` の O(m) 部分: `u_seq.remove` の memmove、`pack_scaled_dense` の
全長走査 ×2、`row_owners` の線形探索と Vec 伸長**
- 対象: `src/simplex/lu.rs:4620-4670` (`self.u_seq.remove(seq_pos)` + `slot_pos` 更新ループ、
  `HybridVec::pack_scaled_dense(e_tilde, …)` と `(a_tilde, …)` = `src/sparse.rs:453-488` の O(m)
  走査、`row_owners[row_step].iter().position(..)` + `swap_remove`、`row_owners[..].push`)。
- 計測: commit_update 命令数 bnl1 5.6%、ganges 4.4%、pilot 2.9%、pilot87 2.5%; うち pack_scaled_dense
  が bnl1 4.0% / pilot 1.5% (`if i != skip && v != 0.0` の行が bnl1 1.8%)。`grow_one` の主呼び出し元
  (pilot 199k 回) は row_owners/rows の push。ft_update フェーズ wall: bnl1 6.7%、pilot87 4%、dfl001 4%。
- 対処 (ビット同一): (a) `u_seq` は tombstone 方式 (slot に `dead` フラグ、FTRAN の逆順/BTRAN の順走査で
  skip、再分解で消える) にして memmove と slot_pos 更新を無くす — 生きている eta の相対順序は同じ。
  (b) `e_tilde`/`a_tilde` の非ゼロ添字リストを BTRAN U^T 段 / FTRAN の permute 段で同時に集め
  (`permute_out` は既に nnz を数えている)、`pack` は添字リストからだけ作る。(c) `row_owners` は
  `Vec<(u32,f64)>` + 予約容量 (前回の長さ ×1.5)。
- 効果: m ≥ 600 の問題で 2〜4% (bnl1, ganges, sctap3, stocfor2, dfl001, pilot87)。難易度: M。

**L6. 単位ベクトル BTRAN の U^T 掃引が seed より前の eta も全部訪問する**
- 対象: `src/simplex/lu.rs:3816-3855` (`u_transpose_sweep`: `singles` + `u_seq` 全走査)、
  `:4325-4340` (`solve_transpose_unit(_capture)`)。`u_transpose_solve_from` (:3857) は再分解直後専用。
- 計測: `u_transpose_sweep` 命令数 pilot87 3.4%、pilot 3.0%、bnl1 2.8% (うち `if zp == 0.0` の空振り
  分岐が bnl1 0.65% + option.rs 0.69%)。btran フェーズ wall は 9〜14%。
- 対処 (厳密): U^T の scatter は「slot p の値が確定 → その owners (u_seq 上で p より後ろ) へ書く」
  なので、seed slot の u_seq 位置 (`slot_pos[seed]`; single なら singles 側) より前の eta は入力・出力
  とも 0 のまま。走査を `slot_pos[seed]` から始める (singles は seed 自身だけ処理)。FT 更新後も
  「off_diag の参照先は常に u_seq 上で前方」の不変条件 (u_solve_into の docs で証明済み) により厳密。
- 効果: U^T 掃引の平均半分 → 全体 1〜1.5% (pilot87/dfl001/bnl1)。難易度: S。

**L7. 5 反復ごとのドリフト検査 `compute_rhs_affine` + `residual_norm_affine` が O(nnz(A)+nnz(B))**
- 対象: `src/simplex/extended_dual.rs:4210-4230`、`:1431-1466`、`:1381-1430`。前監査「未実装: drift
  チェック rhs の増分管理」の再掲。
- 計測: 命令数 bnl1 5.0% (compute_rhs 2.75 + residual 2.25)、pilot 2.3%、ganges 2.2%、pilot87 1.1%+。
- 対処: `rhs = b − N x_N` を BFRT の flip (`std.cols.col(j)` の O(nnz 列)) と基底交換で増分更新し、
  検査時は残差計算だけにする。5 反復ごとの再計算との差は加算順序の丸めだけだが、残差がしきい値
  (1e-8) 付近の反復で再分解判定が変わる可能性がゼロではない → 「ほぼ経路保存、要 A/B」。
  完全にビット同一にするなら検査間隔を変えずに (増分 rhs で) 残差を計算し、再分解トリガ時のみ
  従来通り全再計算する (トリガ判定は増分版の残差になるので厳密には経路変更)。
- 効果: 2〜3% (bnl1, ganges, pilot, 25fv47 など中規模)。難易度: M。

**L8. `refresh_row` / `deviation_core` が Option<Affine1> の傾き比較を毎行フルに実行**
- 対象: `src/simplex/extended_dual.rs:1499-1531` (`deviation_core`: `gt_zero` ×2、`cmp_lex`)、
  `:1612-1623`、呼び出し xb_update `:3912-3935` / BFRT 反映。
- 計測: 命令数 bnl1 6.4%、pilot 4.4%、pilot87 3.4% (`if sa > REL_TOL && sa < INF` 単独で bnl1 1.0%、
  pilot87 0.53%)。CHUZR フェーズ wall は 1〜5% だが refresh_row は xb_update/bfrt 側に計上される。
- 対処 (ビット同一): 行の `lower/upper/x_b_slope` の傾きが全て 0 (M 側列が基底に無い行 = ほとんどの
  行・反復) なら `gt_zero` は `base > 1e-9 && base < INF` に、`cmp_lex` は base 比較だけに帰着する。
  `RowBounds` に `plain: Vec<bool>` (両境界の slope == 0) を持ち、`x_b_slope[i] == 0.0` と併せて
  f64 だけの高速経路を通す (SoA: lower_base/upper_base/has_lower/has_upper)。さらに xb_update で
  `alpha[i]` が非ゼロでも x_B が両境界から十分内側 (前回 deviation なし & 変化量 < 余裕) なら
  再計算不要、は余裕の追跡コストが要るので任意。
- 効果: refresh_row の 4〜6% を半減 → 2〜3% (bnl1, pilot, dfl001, degen3)。難易度: S〜M。

**L9. CHUZC1 の候補を `cand_scratch` に書いてから `candidates` へ extend し、毎反復フルソート**
- 対象: `src/simplex/extended_dual.rs:3118-3145` (`cand_scratch[k] = …` → `kept` → `candidates.extend`)、
  `:3312-3325` (`candidates.sort()`: `Cand` の全順序で安定ソート)。
- 計測: `extend_desugared` が pilot で 1.4% (3,287 回 = 反復数、1 回 38k 命令)、sort は chuzc1 の残り。
  chuzc1 フェーズ wall 3〜5%、fit2d/d6cube/scsd では 17〜30%。
- 対処: `candidates` に直接書き込む (書いてから `keep` で長さを進める現在の分岐なし方式はそのまま)。
  ソートは BFRT が使うのは先頭数個なので `select_nth_unstable` + 部分ソート (全順序が同じなら結果
  同一; 安定性は `Cand` の順序に j が入っているので不要)。
- 効果: 1% (pilot)、fit2d/d6cube/scsd8 で 3〜5%。ビット同一。難易度: S。

**L10. 再分解時の行リスト・バケット・FtLu の Vec<Vec> を毎回新規確保**
- 対象: `src/simplex/extended_dual.rs:1316-1325` (`refactorize`: `vec![Vec::new(); m]` に push)、
  `src/simplex/lu.rs:875-905` (`MarkowitzState::new`: `col_buckets`/`row_buckets` Vec<Vec> ×(m+1))、
  `:3623-3700` (`FtLu::new`: `off_diags`/`row_owners` Vec<Vec> ×m + `HybridVec::pack` ×m)。
- 計測: pilot 命令数の約 5% が Vec 伸長 (`grow_one` 2.2%、`finish_grow` 2.2%、realloc→memcpy 2.1%、
  199k 回)、pilot87 で `finish_grow` 1.3% + memcpy 1.7%; 再分解 1 回あたり pilot87 で約 6k 回の確保。
- 対処 (ビット同一): `refactorize` の行リストは基底列の行数を先に数えて exact 容量で確保 (または
  `RefactorScratch` として反復間で保持し clear して再利用)。`FtLu` の U は平坦 CSR (start/len +
  1 本の (u32,f64) 配列、更新 eta は末尾追記) にして per-slot Vec を無くす。`row_owners` も同様。
- 効果: 再分解比率の高い問題 (pilot87, pilot, grow22, pilot.ja, perold, maros-r7) で 2〜3%。
  難易度: S (行リスト事前確保) / M (FtLu 平坦化)。

**L11. Markowitz `eliminate` の行マージが毎ステップ影響行を書き直す (pilot87 の再分解の 6 割)**
- 対象: `src/simplex/lu.rs:1283-1495` (`eliminate`: `while a < row_len_before && b < plen` のマージ、
  `ents[w] = (ja, new_val)`)。前監査 U4 で in-place 高速経路は入っている。
- 計測: pilot87 `eliminate` 14.0% (総時間比)、pilot 6.4%。マージ行が pilot87 の上位 5 行 (各 0.9%)。
- 対処: (a) 影響行のマージを「ピボット行の列集合を epoch マークしておき、既存要素は in-place 更新、
  fill だけ末尾追記して最後に 1 回ソート」に変える (要素順は列昇順に揃うので同じ; 加算は各要素
  1 回なのでビット同一)。(b) ピボット行の値を密スクラッチ (`ElimScratch`) に散布して影響行側は
  `scratch[j]` を引くだけにする (現在は snapshot と 2 ポインタマージ)。
- 効果: pilot87 3〜5%、pilot/d2q06c/maros-r7 1〜2% (推定)。難易度: M。経路: 加算順序が変わらない
  実装ならビット同一。

**L12. DSE の τ FTRAN が入列 FTRAN より高い (密右辺)** — 対処の余地は限定的だが記録
- 対象: `src/simplex/extended_dual.rs:3664-3700` (融合 FTRAN)。
- 計測: pilot τ 64 µs/iter vs 入列 55 (融合 108)、25fv47 16 vs 12.6。τ は ρ_p (btran_l gather 比率
  pilot87 74% = 密) から必然的に密。
- 対処: 数値経路を変えずに減らす手段は L1 (2 本同時 axpy) のみ。それ以外は経路変更: (a) ρ_p が
  密な反復だけ Devex 重みに切り替える (既存の Devex 実装 `EdgeWeights::Devex` を反復単位で併用)、
  (b) 更新間隔 (合成クロック係数 `ENOMOTO_SYNTH_CLOCK_FACTOR`) を下げて U 段の eta 数を抑える。
- 効果: 不明 (要 A/B)。難易度: M。優先度低。

**L13. polish の準備で摂動付き `d` を全列計算してから、使わずに真の `d` を再計算**
- 対象: `src/simplex/extended_dual.rs:4560-4575` (`d` = BTRAN(c_B 摂動) + O(nnz(A)))、`:4700-4720`
  (`true_d` = BTRAN(c_B) + O(nnz(A)))、加えて `compute_rhs_plain` + FTRAN + `InfeasibleRows::rebuild`。
- 計測: 93 問すべてで polish 反復 0 (`polish_iters=0`) → 摂動付き `d`・`perturb_costs`・x_B の再計算は
  一度も消費されない。kb2 で finish+polish は命令数の 3.2% (うち run_phase 分を除くと約 1%)。
- 対処 (ビット同一): polish 先頭で `true_d` (std.c) を先に計算し、実行不能行が無ければそのまま
  最適判定/handoff へ。摂動付き `d` は polish 反復に入る場合だけ計算する。
- 効果: 小問題 1〜2%。難易度: S。

**L14. primal cleanup `run_phase` が毎反復 x_B を全再計算 (`resync_basics`) し、`d` も BTRAN から再計算**
- 対象: `src/simplex.rs:1757-2213` (`run_phase`)、`Tableau::resync_basics`、`SteepestEdgeState::new`
  (:649)。polish → handoff は `extended_dual.rs:4740-4765`。
- 計測: 53/93 問が handoff。pilot87 では run_phase が **4.3%** (216 反復: `solve_transpose_into` 420 回、
  `resync_basics` 217 回、`solve_into` 216 回 = 1 反復 8.5M 命令、双対主ループの約 1.5 倍)。kb2 2.2%。
- 対処: cleanup の反復を双対ループと同じ増分更新 (x_B は α で更新、`d` は行 α_p で更新) にする。
  あるいは cleanup に入る前に「摂動除去後に双対実行不能になった列」だけを対象にした限定 primal
  (HiGHS の `cleanup` は数反復で終わる) にする。
- 効果: pilot87 3〜4%、他の handoff 問題 (d2q06c, pilot, bnl1 等) 0.5〜3% (推定)。経路: primal 反復の
  丸めが変わるので最終基底が変わり得る (目的関数値は同じ最適解)。難易度: M。

### F. FFI / モデル構築 / ハーネス

**F1. ベンチハーネス `_build_our_model` が highspy 属性を要素ごとに読み二乗オーダー (ours_time 外)**
- 対象: `python/enomoto_solver/benchmark_highs.py:78-100` (`lp.col_lower_[j]`, `lp.col_upper_[j]`,
  `lp.integrality_` の `len()`、`lp.row_lower_[i]` を要素ごとにアクセス — highspy は属性アクセスごとに
  ベクトル全体をコピーする)。
- 計測: 93 問合計 34.9 s (solve 合計 36.2 s と同程度)。dfl001 5.5 s、fit2p 5.7 s、fit2d 3.2 s、80bau3b
  3.0 s、maros-r7 2.9 s、woodw 2.0 s、d6cube 1.1 s。`ours_time` には含まれないが、A/B・全問計測の
  wall を倍にしており、`scripts/ab_bench.py` の 1 プロセス内反復でも毎回払う。
- 対処: 先頭で `col_lower = list(lp.col_lower_)` 等に 1 回だけ取り出す (row_lower/row_upper も)。
- 効果: ベンチ全体の wall -45%。solver 側の数値には無関係。難易度: S。

**F2. `PyModel::add_constraint` の BTreeMap 経由と `solve()` の戻り値**
- 対象: `src/model.rs:41-53` (`to_linear_expr`: 行ごとに BTreeMap を構築)、`src/presolve.rs:127-140`
  (`build_a_g`: BTreeMap を再び Vec に collect)、`src/model.rs:solve` (`x` を Python list へ)。
- 計測: 一括計測の build 時間は F1 が支配的で分離できず。推定: dfl001 (nnz 35k) で BTreeMap 挿入
  ≈ 3〜5 ms (solve の 0.03%)、`x` の list 化 ≈ 0.2 ms。小問題でも solve の 1% 未満。
- 対処: `LinearExpr` を「ソート済み Vec<(usize,f64)> + 重複和」にする (BTreeMap 不要)。効果 <1%。
  難易度: S。優先度低。

## 2. 優先リスト (期待効果 × 手軽さ)

| 順位 | ID | 主な対象問題 | 期待効果 | 経路 | 難易度 |
|---|---|---|---|---|---|
| 1 | P1 faer 並列 OFF (+初回スレッド生成の除去) | 密経路 24 問、単発計測では全問 | 小問題 10〜40%、wood1p の非決定性解消 | 実用上同一 | S |
| 2 | L2 列ミラーに値を持たせ二分探索を排除 | pilot87, dfl001, d2q06c, pilot | 4〜7% | ビット同一 | M |
| 3 | L1 U 段 2 本同時 axpy | 重い 10 問すべて | 3〜5% | ビット同一 | S |
| 4 | P6 presolve の Vec<Vec>↔Csr 往復削減 | 軽い 83 問 | 5〜10% | ビット同一 | M〜L |
| 5 | L3 drift 再分解の緩和 | pilot87, pilot, greenbea, grow22 | 5〜15% | **変わる** | S (+A/B) |
| 6 | L4 BFRT FTRAN の融合 (+x_B 更新統合) | fit2p, fit1p, dfl001 | 5〜13% | (a) 同一 / (b) 変わり得る | M |
| 7 | L8 refresh_row の f64 高速経路 | bnl1, pilot, dfl001, degen3 | 2〜3% | ビット同一 | S〜M |
| 8 | L5 commit_update の O(m) 除去 | m ≥ 600 の問題 | 2〜4% | ビット同一 | M |
| 9 | L11 eliminate の行マージ | pilot87, pilot, maros-r7 | 2〜5% | 同一 (順序維持) | M |
| 10 | L7 ドリフト検査 rhs の増分管理 | 中規模 (bnl1, ganges, 25fv47) | 2〜3% | ほぼ同一 (要 A/B) | M |
| 11 | L14 primal cleanup の増分化 | pilot87 と handoff 53 問 | 0.5〜4% | 変わり得る | M |
| 12 | L10 再分解時の確保削減 | 再分解比率の高い問題 | 2〜3% | ビット同一 | S/M |
| 13 | P2 aggregator の早期 return | 軽い群、agg/scrs8/bandm/tuff/scorpion | 2〜10% | ビット同一 | S |
| 14 | P3 doubleton の早期 return | seba, standata, bore3d, beaconfd | 1〜7% | ビット同一 | S |
| 15 | P4 parallelcols の CSC 再構築回避 | standata/standgub/fit2d | 1〜8% | ビット同一 | S〜M |
| 16 | P5 固定点の早期打ち切り | fit1d, fit2d, agg2, agg3, czprob | 1〜8% | ビット同一 | S |
| 17 | P10 std_form/ColCache/PRICE 行列の構築 | solve < 10 ms の約 30 問 | 2〜5% | ビット同一 | S |
| 18 | P8 scaling の平坦ループ化 | fit2d, seba, bore3d, standata | 1〜6% | ビット同一 | S |
| 19 | L6 U^T 掃引の prefix skip | pilot87, dfl001, bnl1 | 1〜1.5% | 厳密同一 | S |
| 20 | L9 CHUZC1 の直接書き込み + 部分ソート | pilot, fit2d, d6cube, scsd8 | 1〜5% | ビット同一 | S |
| 21 | L13 polish の摂動付き d の遅延計算 | 小問題 | 1〜2% | ビット同一 | S |
| 22 | P7 reduce_inequalities の単項行除外 | scrs8, bandm, agg | 1〜2% | 縮約は同一のはず (要確認) | S |
| 23 | P9 inner rebuild_g/extract の往復回避 | 軽い群 | ~1% | ビット同一 | S |
| 24 | P11 dualpropagate 転置の再利用 | maros-r7 ほか | <1% | ビット同一 | S |
| 25 | L12 τ FTRAN (Devex 併用 / 更新間隔) | 重い問題 | 不明 | 変わる | M |
| 26 | F1 ハーネスの highspy 属性コピー | 全問 (ours_time 外) | ベンチ wall -45% | 無関係 | S |
| 27 | F2 add_constraint の BTreeMap | — | <1% | ビット同一 | S |

「各問題で 10% 以上」を狙う観点では、軽い群は P1 + P2 + P3 + P4 + P5 + P6 + P10 の組で
presolve/固定費 (solve の 30〜60%) を 3 割前後削れば達成圏、重い群は L1 + L2 + L5 + L8 + L10 + L11
(すべてビット同一) で 10〜15%、経路変更を許すなら L3/L4(b) を足して pilot87/fit2p で 20% 超が見込み
(いずれも推定)。

## 3. 補足: 今回見送った/計測で否定されたもの

- `env::var` のホットパス読み: MarkowitzState::new (5 回/再分解) や FtLu::new (2 回) にあるが 1 回
  1 µs 未満で全体 0.1% 未満。対処不要。
- polish 本体の反復コスト: 93 問すべてで 0 反復のため主ループとしては無視できる (L13 の準備コストのみ)。
- `HybridVec` の dense 判定 (`DENSE_ETA_FRACTION = 0.4`): dense アームは連続 axpy で自動ベクトル化
  されており、閾値変更の効果は不明 (計測せず)。
- BTRAN L^T の gather/scatter 判定 (`BTRAN_L_SCATTER_FRACTION = 0.10`): pilot87 は 74% gather。
  gather 側の `w[row_step] == 0.0` 分岐が pilot87 1.65% だが、ρ が密なので構造的。
- ピボット探索上限 (256) と閾値: 既存分析 (`pivot_search_limit_20260922_143000.md`) の通り経路変更に
  なるため、ここでは L2 (同じ探索を安く行う) を優先。
