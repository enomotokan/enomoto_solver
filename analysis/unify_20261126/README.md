# unify: 同じ働きのコードの重複を洗い出し、まとめて磨く (2026-11-26、親の依頼、GATE)

利用者の指示は「LP に限らず、別の箇所で共通する部分は一つにまとめて、そのコードを磨くように」。

- 版: 7a1ab08bd で洗い出した。cargo test は 424 通過 (無視 19)。
- 調べ方: src 全体を 3 つの区分 (固定 LP と LP の作り直し、伝播と実行可能性、期限・環境変数・FJ の準備) に分けて読んだ。主な行は手で開いて確かめた。
- 行番号は 7a1ab08bd のもの。ほかの子が同時に src を変えているので、ずれる。

## 0. 方針

- 動き (道筋) を変えないまとめから、1 件ずつ小さなコミットにする。毎回 check_default と cargo test を確かめ、区切りごとに fuzz 7 本も回す。
- 写し違いでバグらしい所 (2 節) は、直す前に親へ報告する。直すなら既定オフの切り替えにして、P で測ってから決める。
- ほかの子の作業中の所 (FMjE の研磨・根のヒューリスティクス、HJZ2 の LP の速さ、Gm3w の rootpar_cut など) は、一覧に載せるだけにするか、後回しにする。

## 1. 重複の一覧 (まとめる値打ちの大きい順)

「道筋」はまとめたときに動きが変わるかどうか。「同じ」は演算の順まで同じにできるもの。

| 順 | 中身 | 場所 (7a1ab08bd) | 同じ所・違う所 | 違いは意図か | 道筋 | 備考 |
|---|---|---|---|---|---|---|
| 1 | 研磨 3 関数の「整数の固定・書き戻し・採るかの判定」 | mod.rs:907 `polish_in_problem`、1003 `polish_in_problem_warm`、1083 `polish_solution` | 判定式 `(vn<=vo.max(1e-9) && zn<=z+1e-9·scale) \|\| vn<vo` が 3 か所。`polish_solution` だけ STRICT が切れていると 1e-6 の上限が付く。解き方は `solve_lp` (前処理つき) と TwoStageLp で別 | 判定は意図して写したもの。解き方の違いも意図的 | 判定と固定だけなら同じ。解き方まで 1 本にすると変わる | **FMjE の担当 (研磨)**。warm が TwoStageLp を作り直す件 (dano3_3 で 20 秒) も FMjE が試作中。ここは触らない |
| 2 | 1 本の行の最小活動度と範囲の締め | solver.rs:24218 `cutpool_prop_cut`、24291 (CUTPOOL_PROP_FULL)、24385 `propagate_le_row_ex`、25551 `row_prop`、17229 `global_obj_prop_ex`、24814 `apply_dual_proofs`、fixmore.rs:52 `obj_row_pass` | 7 か所とも `b = c>0 ? lo : up`、無限の項を数え、2 つ以上なら抜ける。締めの余裕 (need・slack・丸め) と矛盾の許容誤差は場所ごとに違う | 24218 と 24291 は純粋な写し。fixmore は「global_obj_prop と同じ」とコメントしながら整数の slack が違う (写し違い) | 写し 2 件は同じ。ほかは余裕を引数にすれば同じ | 毎ノード通る。約 300 行 |
| 3 | 期限「今から secs 秒、ただし求解の期限まで」の手書き | separation.rs 11 件、solver.rs 8 件、heur_ipm.rs:192、crossover.rs:3075 (計 21 件) | `now + from_secs_f64(x)` を `self.deadline` と min。x の押さえが `.min(1e6)`・`.max(0)`・なしでばらばら | 写しのばらつき。separation.rs:3785 は押さえ忘れ (2 節) | 有限で正の値なら同じ | `deadline_in(secs)` 1 つで済む |
| 4 | 期限までの残り秒の手書き | solver.rs 13 件、heuristics.rs 6 件、heur_scip.rs 2 件など (計 23 件) | `deadline.map_or(INF, \|d\| d.saturating_duration_since(now).as_secs_f64())` と match の形 | 式は同じ。`+2.0` や None のときの値だけ違う (意図的) | 同じ | `secs_left()` |
| 5 | B&B の LP (`self.lp`) で整数を固定して解く | solver.rs:8380 `resolve_continuous`、heuristics.rs:3695 `fixed_int_lp`、solver.rs:8530 `try_lp_solution`、heuristics.rs:284 (RR) | `save_state` → 固定 → `solve` → 値を読む → `restore_state`。8403-8456 と 3700-3725 はほぼ一字一句同じ。連続列の範囲の出どころ (p・大域・局所) が違う | 範囲の違いは意図的。`try_lp_solution` だけ費用を戻さず、反復を累計に入れない (2 節) | 範囲を引数にすれば同じ | `fixed_int_lp` から入ると、ほぼ同じ固定 LP を 2〜3 回解く (性能、2 節) |
| 6 | サブ MIP の `MipParams` を全フィールドで書く | heuristics.rs:1286, 2192、heur_scip.rs:406, 713, 1020、mod.rs:5251 | `submip: true, restarts: 0, threads: 1, …` が同じ。gap・cutoff・時間の上限が違う | 違いは意図的 | `MipParams::submip_base(seed)` と `..` で同じ | 約 90 行が約 30 行になる |
| 7 | `min_relaxation` と `repair` | heur_scip.rs:372-442 と 656-747 | 違反量の列を足す、`from_rows`、残り時間、`MipParams`、後始末がほぼ逐語。足す行、固定、上限が違う | 違いは意図的 | 同じ | 約 45 行 |
| 8 | ヒューリスティクスの違反行の集合と活動度の初期化 | 集合 (swap_remove): heuristics.rs:1603, 1872、heur_shift.rs:56、localmip.rs:162。活動度の初期化: 9 か所 | 集合の 15 行は 4 か所で一字一句同じ。違反量の許容誤差は場所ごとに違う | 集合は写し。許容誤差の一部は写し違い (2 節) | 集合と初期化は同じ。許容誤差を揃えると変わる | 約 120 行 |
| 9 | FJ・Local-MIP に渡す問題と範囲 | solver.rs:10891, 10906, 10950, 27010, 27137, 27198、heuristics.rs:1379、heur_scip.rs:571, 632 | `match self.orig { Some(o) => (o, o.col_lo/up), None => (p, dom.lo/up) }` | 27010 の `restarts==0` は意図的。10906 と 27198 は orig がないとき `p.col_lo/up` (根で締まった範囲を落とす。写し違いの疑い) | 同じ (27198 を揃えると変わる) | `fj_problem()` |
| 10 | 問題全体の最小・最大活動度 | mod.rs:10229, 10301, 10478, 3674, 3873、localmip.rs:1302 | 初期化の 8 行が写し。無限を積で見るか境界で見るかが違う | mod.rs:3705 と 3904 の丸め (1e-6 と 1e-9) は写し違いらしい | 同じ順なら同じ | 前処理で 1 回だけ通るので、速さの得はない |
| 11 | `TwoStageLp` の尺度をかけた写し | mip_lp.rs:690 `attach_tw`、2170 `ipm_scale`、2219 `solve_scaled` | 同じ 7 行が 3 回 | 意図的な重複 | ビット単位で同じ | HJZ2 (LP の速さ) の領分なので後回し |
| 12 | 問題から LP モデル (`VariableData` / `ConstraintRow`) を組み直す | mod.rs:914, 3477、solver.rs:9694、mod.rs:5455 | 行を Eq/Ge/Le の順で作る同じ形 | 意図して写したもの | 同じ順なら同じ | model.rs と fj.rs は行の順が違うので含めない |
| 13 | 環境変数の読み取り | `OnceLock` の中で `env_str!` を読む二重キャッシュ 55 件、`env_str!(..).and_then(parse).unwrap_or(..)` で `tunable!` と同じもの 16 件 | — | 冗長なだけ | 同じ | 「0 で切る」の意味の割れ (`.is_some()` 739 件と `!= "0"` 97 件など) は、揃えると `=0` を付けたときの意味が変わるので触らない |
| 14 | 小さな重複 | B&B の LP を作り直す (solver.rs:6757, 6791 など)、基底の `set_basis_nb` か `set_basis` かの分岐 (8 か所)、行の照らし合わせの HashMap (3 か所)、FJ と Local-MIP の交互回し (heuristics.rs:4253 と prechain.rs:358)、目的の打ち切り行 (heuristics.rs:1268 と 1336)、改善の判定 `z < best - 1e-9·max(\|best\|,1)` (10 か所)、整数の範囲の丸め (5 か所) | — | ほぼ意図的 | ほぼ同じ | 交互回しは時計が違う (手間の時計か壁時計か) ので、揃えると prechain の道筋が変わる |

## 2. 写し違いでバグらしい所 (直す前に親へ報告。直すなら既定オフで P を測る)

| 順 | 場所 | 中身 | 影響 |
|---|---|---|---|
| 1 | heuristics.rs:1437 (`local_search`)、1781 (`fj_search_old`) | 違反の許容誤差が `FEASTOL·(1+min(\|act\|,1e6))` で、`is_feasible` の `1e-6+1e-9\|b\|` より緩い。localmip.rs:147 は「is_feasible より狭く」と明言している | \|act\| の大きい行では、`is_feasible` が拒む点を実行可能とみなす。`local_search` は前の本物の解 `best_feas` を上書きしうる。1721 の `better` は目的値だけで比べるので、旧 FJ の偽の解が FJ2 の本物の解を押しのけうる |
| 2 | heur_scip.rs:665 (`repair`) | 違反なしの判定が `1e-6(1+\|b\|)` | \|b\| が大きい行の違反を「なし」と数え、`try_incumbent` で落ちても補助 MIP を試さない |
| 3 | separation.rs:3785 (`root_cut_loop_inner`、ROBUST の `d`) | `from_secs_f64(2.0*cap)` を作ってから `cap.is_finite()` を見ている | 時間の上限が無限だとパニックする。試作の ROBUST の `d` のときだけ通る。同じ関数のほかの所は先に確かめている |
| 4 | solver.rs:8530 (`try_lp_solution`) | 兄弟の 2 つと違い、費用を元に戻さず、反復を累計に入れない | FP の中では距離の費用のまま連続列を解く。後で `resolve_continuous` が直すので、実害は主に反復の予算の数え方 |
| 5 | fixmore.rs:78 と solver.rs:17255 | 「同じ」とコメントしているのに、整数列の slack (`1e-9(1+\|lim\|)`) が fixmore にだけ付く | 結果が変わるのは範囲の際だけ |
| 6 | mod.rs:3705 と 3904 | 丸めの余裕が 1e-6 と 1e-9。FZ2_FIX で片方だけ揃えたように見える | 対称の行を足すとき |
| 7 | mod.rs:6560 と 6875 (lp_probe から根の LP へ基底を渡す) | ROOTCAP で止まった道だけ、`ROOT_BASIS_SAVED` を置かず、DSE を条件なしで渡す | その道では `root_basis_rebuild` が使えない |
| 8 | solver.rs:10906、27198 | orig がないとき、FJ・Local-MIP の範囲に `p.col_lo/up` を使う (ほかは `dom.lo/up`) | 根で締まった範囲を落とす。27198 は LMBOOST、10906 は診断 |
| 9 | mod.rs:3613 (`postsolve_diag`、診断) | 許容誤差に `max(\|lo\|,\|up\|).min(1e300)` を使う | 片側の行では許容誤差が約 1e291 になり、違反を報告しない |
| 10 | mod.rs:856 (`max_violation`) | `f64::max` が NaN を無視する | NaN を含む点の違反が 0 になる (`is_feasible` は有限でない値を拒む) |
| 11 | heuristics.rs:1429, 1765 | `time_cap` に `.max(0.0)` がない (fj.rs:345 と localmip.rs:1215 にはある) | 負の値でパニックしうる |
| 12 | 研磨の性能 (1 節の 1、5 の関係) | `fixed_int_lp` → `try_incumbent` → `resolve_continuous` (→ warm 研磨) で、ほぼ同じ固定 LP を 2〜3 回解く | 入口は heur_shift の IntShift だけ |

## 3. まとめの記録

(1 件ずつ足していく。確かめは毎回、cargo test・`pip install -e .`・check_default の順)

### 3.1 期限のヘルパ `deadline_in(secs)` と `secs_left()` (1 節の 3・4)

- `Solver` に 2 つのヘルパを置いた (solver.rs の `time_up` の前)。
  - `deadline_in(secs)`: 今から `secs` 秒後と `self.deadline` の早い方。秒を [0, 1e6] に押さえる (負・NaN は 0、無限・大きすぎる値は 1e6)。
  - `secs_left()`: 期限までの残り秒 (期限がなければ無限)。
- 置き換えたのは 34 か所。
  - 期限の手書き 15 か所: separation.rs 9、solver.rs 4、ほか。
  - 残り秒の手書き 19 か所: solver.rs 11、heuristics.rs 6、heur_scip.rs 2。
- 手で残したもの:
  - separation.rs:1456: 手間の期限 (`tk_c` で 10 倍) と `fit_d` を重ねる形。
  - solver.rs:7170: `tk_m = false` で使われていない道。
  - Solver の外のもの (crossover.rs など)。
- 動き: 有限で正の秒なら、前と同じ計算。`.min(1e6)` を付けていた所は、ヘルパの中の押さえで同じになる。
- 直った所: 2 節の 3 (separation.rs:3785) が、秒を押さえるので落ちなくなった。
- 確かめ: cargo test 424 通過、check_default 全部一致。

### 3.2 整数を固定した LP を B&B の LP の上で解く `with_fixed_int_lp` (1 節の 5)

- `resolve_continuous` (solver.rs) と `fixed_int_lp` (heuristics.rs) の共通部分を `Solver::with_fixed_int_lp` にまとめた。共通部分は次のとおり。
  1. 状態を取っておく。
  2. 費用を元に戻す。
  3. 整数列を固定する。
  4. 解く。
  5. 整数列を固定値に置き直す。
  6. 状態・費用・反復の累計を戻す。
- 違いは引数で渡す。
  - 丸めるか: `round`。`fixed_int_lp` は丸める。
  - 連続列の範囲: `cont`。`resolve_continuous` は問題の境界、`fixed_int_lp` は大域の境界。
  - 期限。
  - 最適のときの処理: `on_opt`。LP が解けた状態のまま呼ぶので、`resolve_continuous` の漏れの見積りと基底の取り置きもその中でできる。
- 磨いた所: `fixed_int_lp` は `x[j].round()` を 2 回計算していたが、`resolve_continuous` と同じ流れにそろえた。
- 動き:
  - LP の演算は同じ。
  - `resolve_continuous` の期限の計算は、前は列の境界を置いた後、今は前。差はマイクロ秒で、期限の上限が効くのは試作の `INC_LP_CAP`・`FJ_P3` のときだけ。
- `try_lp_solution` と RR は入れていない。`try_lp_solution` は費用を戻さず反復を累計に入れる (2 節の 4)。そろえると動きが変わるので、別に扱う。
- 確かめ: cargo test 424 通過、check_default 全部一致。

### 3.3 サブ MIP の設定の土台 `MipParams::submip_base` (1 節の 6)

- `MipParams::submip_base(time_limit, node_limit, seed)` を置いた。中身は、ギャップ 0 (絶対 1e-9)、打ち切り値なし、1 スレッド、経過を出さない、`submip: true`、`tl_base: NaN`。
- 全フィールドを書いていた 6 か所を、土台 + 違う所だけの形にした。
  - heuristics.rs: `fp_stage3`、`solve_submip_problem`
  - heur_scip.rs: `min_relaxation`、`repair`、vb01
  - mod.rs: `clqprb_shadow_locks`
- 動き: どのフィールドの値も前と同じ。
- 確かめ: cargo test 424 通過、check_default 全部一致。

### 3.4 `min_relaxation` と `repair` の共通部分 (1 節の 7)

- heur_scip.rs の 2 つの補助 MIP のヒューリスティクスから、共通部分を 3 つの関数に出した。
  - `slack_relaxed(p, col_lo, col_up, relax)`: 違反量の列を足した問題を作る。足す行は `relax(i)` で選ぶ (`min_relaxation` は全部、`repair` は違反行だけ)。列は下限側・上限側の順で、前と同じ。
  - `solve_slack_relaxed`: サブ MIP の設定で解き、反復を数える。
  - `finish_slack_point`: 点を暫定解として試し、だめなら整数列の値を目標に固定・伝播する。
- 磨いた所: 違反量の合計は経過を出すときだけ計算する。
- 動き: 同じ。
- 確かめ: cargo test 424 通過、check_default 全部一致。

### 3.5 カットの貯めの伝播の全部を回す道を `cutpool_prop_cut` に寄せる (1 節の 2 のうち純粋な写し)

- 対象は `propagate_cut_pool_inner` (`ENOMOTO_MIP_CUTPOOL_PROP_FULL`、既定では通らない)。差分版の `cutpool_prop_cut` と一字一句同じ 50 行の写しを持っていたので、呼び出しに置き換えた。
- 締めたカットの年齢を 0 に戻すのは、前から差分版だけだった。これを引数 `reset_age` で残した。
- 消したもの: 写しの中にあった、特定のカット 1 本 (`k + 738 == 本数`) を出す診断の行 (`ENOMOTO_MIP_DEBUG_CPP`)。入口の診断の行は残した。
- 動き: 同じ (既定の道は変わらない)。
- 確かめ: cargo test 424 通過、check_default 全部一致。

### 3.6 安全の直し (2 節の 3・10・11、親の指示でまとめのついでに直す)

- `crate::cancel::secs(x)` を置いた。秒を [0, 1e6] に押さえた `Duration` を返す (`Duration::from_secs_f64` は負・NaN・無限で落ちる)。
  - 次の 4 か所の期限の作り方をこれにした。
    - `Solver::deadline_in`
    - heuristics.rs の `local_search` と `fj_search_old` (前は押さえなし、2 節の 11)
    - fj.rs と localmip.rs (前は `.max(0.0)` だけで、無限で落ちた)
- `max_violation` (mod.rs): NaN の違反を無限として扱う (2 節の 10)。前は `f64::max` が NaN を捨てて、NaN を含む点の違反が 0 になり、研磨の採否で通りえた。
- 動き: 有限の値では同じ。
- 確かめ: cargo test 424 通過、check_default 全部一致。

### 3.7 履歴で確かめたこと (2 節の 4・7、親の指示)

- 2 節の 4 (`try_lp_solution` が費用を戻さず、反復を累計に入れない):
  - `try_lp_solution` は 2026-10-06 の分枝限定法の骨組み (008552d28) からある。
  - 費用を元に戻す処理と、反復を累計に入れない処理は、2026-10-15 に `resolve_continuous` を足したとき (9ba979b42) に、そちらにだけ入った。そのとき `try_lp_solution` は見直されていない。意図した違いではない。
  - 揃えると動きが変わるので、4 節の切り替え (`t`) に入れて P で測る。
- 2 節の 7 (lp_probe の ROOTCAP の道で `ROOT_BASIS_SAVED` を置かない、DSE を条件なしで渡す):
  - 試作 `ENOMOTO_MIP_ROOTCAP` の `p`・`q` (e3fd57dc3、既定では切) の道で、コメントに「止まった LP の基底 (行の状態・DSE 重みつき) を捨てずに根の LP に渡す (根の LP はここから続きを解く)」とある。
  - DSE を渡すのは意図的。`ROOT_BASIS_SAVED` は別の試作 (`ROOT_BASIS_FIX`) の置き場で、この道には関わらない。
  - 写し違いではないので、直さない。

## 4. 写し違いの直し `ENOMOTO_MIP_UNIFY_FIX` (既定オフ、P で測る)

親の指示で、動きが変わる直しは正しい側にそろえる形で切り替えにし、まとめて 1 つの P で測る。文字ごとに入れ、`1` は全部。

| 文字 | 2 節 | 直し |
|---|---|---|
| `f` | 1 | `local_search`・`fj_search_old` の違反の許容誤差を `0.5e-6 + 1e-10\|b\|` (Local-MIP の `viol_at` と同じで、`is_feasible` より少し狭い) にする |
| `r` | 2 | Repair の「違反なし」の判定を `is_feasible` と同じ `1e-6 + 1e-9\|b\|` にする |
| `s` | 5 | FIXMORE の `obj_row_pass` で整数列に丸めの余裕を足さない (`global_obj_prop_ex` と同じ。FIXMORE は既定では使わないので P では効かない) |
| `m` | 6 | 対称な列の併合の `sum_ub` の余裕を 1e-6 にする (`merge_twin_classes` の `FZ2_FIX` の `m` と同じ) |
| `l` | 8 | LMBOOST の Local-MIP に、元の問題がないとき大域の境界を渡す |
| `t` | 4 | `try_lp_solution` の固定 LP を `with_fixed_int_lp` で解く (元の費用、反復は累計に入れない、連続列はノードの境界) |

- 既定オフのときの確かめ: cargo test 424 通過、check_default 全部一致。
- P: 58 問 × 種 1〜3 × (def_t1・def_t4・uf_t1・uf_t4)、60 秒、696 回。台本は analysis/polishp_20261125 の写し (cfg.py だけ差し替え)。
  - 分担表: `python plan.py 4 shards.json` (4 分担、各 1.6 時間の見積り)。
  - 回し方: `python analysis/unify_20261126/run.py K` → res_sK.jsonl。
  - 集計: `python analysis/unify_20261126/summ.py`。

### 4.1 fuzz (`ENOMOTO_MIP_UNIFY_FIX=1`、`FUZZ_VENV=/usr/local bash analysis/unify_20261126/fuzz.sh uf ENOMOTO_MIP_UNIFY_FIX=1` → fuzz_uf.out)

- 7 本は誤答 0。
  - fuzz_tiny 0/400・0/400
  - fuzz_highs 0/300
  - fuzz_objint 0/1000・0/1000
  - fuzz_slack は mismatches 0 (1000・1000 回)
- cargo test は 424 通過。
- 新しく足した fuzz_bigact.py: 活動度の大きい行 (係数に 10^2〜10^4 を掛け、|act| が 1e3〜1e6) の小さな MIP で、HiGHS との突き合わせと、返した点の実行可能性 (`1e-6 + 1e-9|b|`) を見る。
  - 2 節の 1 (旧 FJ の緩い許容誤差) の誤答の芽を見るためのもの。
  - 結果は 0/300・0/300。
  - seed 88 は、こちらが -15350.21 で HiGHS が -15310.65 だった。こちらの点は厳密に実行可能 (違反 0) で、HiGHS はギャップ 1e-12 でも同じ値なので、HiGHS の取りこぼし。既定でも直しでも同じ値。判定は「HiGHS より良く、点が実行可能なら印字だけ」にした。

### 4.2 P の結果 (696 回、4e0e49723 のビルド、`summ.txt`)

- 分担: 0 は GATE、1 は HsTC、2 は Fnaw、3 は GcYE。4 人とも 4e0e49723 を取り出してビルドし、check_default は全部一致。
- crash・取得失敗は 0、食い違いは 0。
- 既定との比 (`uf` ÷ `def`):

| スレッド | 比 | 95% 区間 | 解けた数 (既定 → 直し) | 全種 1.25 倍超の遅れ | 目安 |
|---|---:|---|---|---|---|
| 1 | 1.002 | [0.998, 1.005] | 124 → 124 | なし | 候補 |
| 4 | 0.987 | [0.965, 1.009] | 139 → 138 | なし | 外れ (解けた数) |

- 4 スレッドの解けた数の差は mzzv42z の種 3 の 1 回だけ。既定は 58.9 秒で解け、直しは 60 秒の時間切れで、上限の際の差。
- 分担ごとの報告で目立った neos-911970 の 4 スレッドの遅れ:
  - 4 スレッドでは種 1〜3 で 1.07・1.54・1.63 倍だった。1 スレッドでは 1.06・1.03・1.02 倍で、ほぼ同じ。4 スレッドの揺れの範囲と見る。
- 効いた問題: noswot の 4 スレッドで 0.56 倍。
- 見立て: 中立。親の方針 (中立なら入れる) に合う。

### 3.8 1 本の行の伝播を `Domain` の 2 つの関数にまとめる (1 節の 2)

- `Domain::tighten_le_row(p, row, u, cont_frac, int_only, before)`: 最小活動度・矛盾の判定・`gap` で締める処理。次の 2 つの共通部分。
  - カットの貯めの伝播 `cutpool_prop_cut`
  - 目的の行・被約費用の行の伝播 `propagate_le_row_ex` (毎ノード通る)
  - 違いは引数で残した: 連続列の締めの下限 `cont_frac` (カットの貯めは 0.1、目的の行は 0)、`int_only`、締める前に印を置く `before`。
- `Domain::tighten_le_row_rest(p, row, u, int_slack)`: 「ほかの列の最小活動量 `rest` から `c_j x_j <= u - rest`」で締める処理。次の 2 つの共通部分。
  - 大域の目的の伝播 `global_obj_prop_ex`
  - FIXMORE の `obj_row_pass`
  - 整数列の丸めの余裕の違いは `int_slack` で残した (FIXMORE だけ足す。`UNIFY_FIX` の `s` で足さない)。
- 残したもの: `row_prop` (`tc_global_pool_prop`) と `apply_dual_proofs` の伝播。締め方 (自前の丸め・絶対値の許容誤差) が違うので入れていない。
- 動き: 演算の順は同じ。
- 確かめ:
  - cargo test 424 通過、check_default 全部一致。
  - 速さ (`nps.py`): 伝播の重い P の 11 問を、1 スレッド・種 1・60 秒・各 2 回、前後で別々にビルドして比べた (`nps_base.jsonl`・`nps_u8.jsonl`)。
    - 解けた 7 問はノード数が前後で同じ (道筋が同じ)。
    - nodes/s の比 (後/前) の幾何平均は 0.995。問題ごとに 0.93〜1.06 で、同じ設定の 2 回の間の揺れ (mas76 で 23.8〜27.0 秒) の内。速さは変わらない。

| 問題 | 前 ノード・秒 (2 回) | 後 ノード・秒 (2 回) | nodes/s の比 (後/前) |
|---|---|---|---:|
| gen-ip054 | 267295・60.0、262860・60.0 | 234632・60.0、263091・60.0 | 0.939 |
| mas74 | 305057・60.0、294804・60.0 | 292328・60.0、305689・60.0 | 0.997 |
| mas76 | 121341・25.1、121341・23.8 | 121341・25.3、121341・27.0 | 0.934 |
| mik-250-20-75-4 | 5856・11.4、5856・11.9 | 5856・12.5、5856・11.2 | 0.988 |
| misc07 | 4330・5.6、4330・5.1 | 4330・6.1、4330・4.9 | 0.981 |
| neos5 | 151929・60.0、143945・60.0 | 146309・60.0、148253・60.0 | 0.996 |
| noswot | 17457・13.8、17457・14.6 | 17457・15.9、17457・12.9 | 0.998 |
| pk1 | 223081・60.0、220755・60.0 | 216012・60.0、220311・60.0 | 0.983 |
| qiu | 3787・20.0、3787・21.9 | 3787・20.0、3787・19.7 | 1.056 |
| rout | 6752・16.9、6752・17.1 | 6752・16.3、6752・16.0 | 1.051 |
| timtab1 | 20421・51.2、20421・54.5 | 20421・52.1、20421・50.0 | 1.035 |

### 4.3 既定にした (2026-11-26、利用者の承認、親の指示)

- `ENOMOTO_MIP_UNIFY_FIX` の既定を `1` (全部) にした。`=0`・`off` で以前の形に戻る。
- 確かめ (既定のビルド):
  - check_default 全部一致: 6 問の道筋は変わらないので、基準 ref_default.json はそのまま。
  - cargo test 424 通過。
  - fuzz 7 本 (`fuzz_def.out`) 誤答 0。
  - fuzz_bigact 2 本 0/300・0/300。seed 88 は HiGHS の取りこぼしで、4.1 節と同じ。
- fuzz.sh を環境変数なしで回すと、最後の cargo test の行が `$2` 未設定で止まっていたので直した (cargo test は上の確かめで別に回している)。
- docs/improvement_history.md に記録した。
