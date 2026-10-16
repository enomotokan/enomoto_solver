# MIP が解も下界もないまま `not_solved` で止まる問題の調査 (2026-10-16)

対象: MIPLIB 2017 benchmark の leo1・leo2・neos-3656078-kumeu・neos-933966 (ENOMOTO が `not_solved`) と momentum1
(計測スクリプトが MPS を読めなかった)。HiGHS はどれも 60 秒で正常に時間切れ。

- 環境: Linux x86_64、4 コア。`cargo build --release --lib` の lib_core.so を `_core<EXT_SUFFIX>` として
  python/enomoto_solver/ の .py と並べ、PYTHONPATH で読む。highspy 1.15 系
- 再現: develop の最新 (31740cc) で 4 問とも再現した (b38653b からの変化ではないので、b38653b での確認は省いた)。
  スレッド数 1 でも同じ (leo1 は 0.38 秒で `not_solved`。ただし Feasibility Jump の解 7.24e8 は持つ)
- 修正: ブランチ `claude/mip-notsolved-leo-kumeu`

## 問題ごとの原因

### leo1・leo2: 根の LP が二段解法の仕上げで `Error` (どの `NotSolved` か: solver.rs の根の LP の `_ =>`)

`ENOMOTO_MIP_DEBUG_LP=1 ENOMOTO_DEBUG_RACE=1 ENOMOTO_DEBUG_EXT_ITERS=1` で見ると:

```
DEBUG_EXT: main_loop_iters=428 ... z=(388573315.509608,0)        <- 双対単体法は正しい最適値に着いている
DEBUG_EXT: polish DUAL->PRIMAL cleanup handoff at polish_iter=0
DEBUG_EXT: primal_handoff_us=259960 dual_infeasible_cols=12 primal_handoff_iters=1319
DEBUG_EXT: primal_handoff_us=344 dual_infeasible_cols=1 primal_handoff_iters=1   (2 回目・3 回目も同じ 1 列)
DEBUG_EXT: polish handoff limit (3) reached; still dual infeasible -> NotSolved
TSLP solve st=Error iters=641 ...
MIP root race: simplex won at 0.097s (Error)
```

- 仕上げ (`polish_with_true_bounds`) は真の費用での被約費用を絶対の許容 `TOL = 1e-9` で判定する。残った 1 列は
  d_j = -2.44e-9、費用 c_j = 3.14e7 (相対 1e-16、丸め誤差そのもの)。主単体法への引き継ぎ (主単体法の許容ではこの列は
  最適) では消えず、引き継ぎの上限 3 回で `None` → `LpStatus::Error`
- さらに同時実行 (`TwoStageLp::solve_race`) は、二段解法が `Error` で戻っても「先に終わった」として内点法側を止めていた
- 根の LP だけを LP として解くと (整数性なし、`model.solve`) 0.07 秒で 388573315.5096 (HiGHS 388573315.5096、280 反復)。
  leo2 も 386421293.2089 で一致。LP の経路 (別の前処理・尺度) では起きず、MIP の前処理後の LP で起きる
- 並列時は Feasibility Jump のスレッドの解も受け取らずに終わるので、解もなかった

### neos-3656078-kumeu・neos-933966: カットの分離の「面の LP」が期限まで止まらない (`cut_ok` が偽の `NotSolved`)

根の LP は解けている (kumeu -18413.2 / 2.4 秒、neos-933966 318 / 5.6 秒。整数性なしの LP は HiGHS と一致)。
その後、根のカットの最初のラウンドで 56 秒を使い、カットを外した LP も期限切れ (`TimeLimit`) → `root_cut_loop` が偽 →
`NotSolved`。

- カットを加えた LP には 1 回の時間の上限 (全体の 2%、最低 1 秒) があるが、実際に時間を使っていたのは分離の中の
  **面の LP** (`separate_face_split`: 最適面の上で副目的を変えて別の頂点を探す LP) で、期限が全体の期限 (`self.deadline`)
  だけだった。一時的な出力で確かめた (カットの LP を解く時点で残り時間 0)
- kumeu の面の LP は特異基底・主単体法の引き継ぎの破綻 (`max|x_B| 1.9e8`) で解き直しを繰り返していた (数値的に悪条件)
- 期限で終わったのに `NotSolved` と報告していた (状態の対応づけの誤り)

### momentum1: 計測スクリプトの読み込み

`benchmark_highs._load_lp` が `"kOk" not in str(status)` で失敗としていたが、momentum1 は `HighsStatus.kWarning`
(読み込みはできている)。

## 直し方

| # | 箇所 | 内容 |
|---|---|---|
| 1 | `slope_intercept_dual.rs` `polish_with_true_bounds` | 双対実行可能性の判定の許容を列ごとに `max(TOL, 64 ε (|c_j| + Σ|a_ij y_i|))` にする (被約費用の丸め誤差の大きさ。`ENOMOTO_T_POLISH_DUAL_ROUND_K`、0 で従来どおり)。費用が 7e4 程度より小さい列では従来と同じ。**leo1・leo2 の根本原因の修正** |
| 2 | `mip_lp.rs` `solve_race` | 二段解法が `Error`・`IterationLimit` で終わったら勝ちとせず、期限まで内点法 + クロスオーバーの結果を待つ。内点法側にも期限を渡す (`cancel::with_deadline`) |
| 3 | `mip_lp.rs` `solve_ipm_rescue`、`lp_api.rs` `MipLp::solve_rescue`、`solver.rs` | 根の LP が `Error`・`IterationLimit` なら内点法 + クロスオーバーだけで解き直す (スレッド数 1 でも)。それでも解けなければ Feasibility Jump の解を受け取り (`join_fj_thread`)、期限なら `time_limit`、そうでなければ `not_solved` |
| 4 | `separation.rs` `separate_face_split` | 面の LP 1 回にもカットを加えた LP と同じ時間の上限 (全体の 2%、最低 1 秒)。**kumeu・neos-933966 の修正** |
| 5 | `solver.rs` カットのループの失敗 | 期限に当たっていれば `time_limit` (下界はカット前の根の LP の値)。Feasibility Jump の解も受け取る |
| 6 | `solver.rs` | 根の LP の値を根の下界として記録する (カットのループが途中で終わっても `root_bound` が残る)。期限を過ぎたら根のヒューリスティクスを走らせない (kumeu で期限後に局所探索が 3 秒走り 63.6 秒になっていた) |
| 7 | `benchmark_highs.py` `_load_lp` | `kError` のときだけ失敗にする |

2・3 は 1 があれば leo1・leo2 では使われない (守りの経路)。`ENOMOTO_T_POLISH_DUAL_ROUND_K=0` (1 を無効) で動くことを
確かめた: 同時実行は `simplex ended without a conclusion ..., waiting for ipm_crossover` で内点法側を待ち、解き直しも
行う。ただし leo1 では内点法 + クロスオーバーの仕上げも同じ判定で失敗するので、このときは Feasibility Jump の解
7.24e8 を持って `not_solved` で返る (以前は並列時は解もなかった)。

Python 側の status の文字列は変えていない (`time_limit` / `not_solved` の意味は model.py の説明どおり。期限で終わったものが
`time_limit` になるだけ)。scripts/ の計測は `optimal` だけを解けたと数え、`time` を含む状態を `t` と表示するので影響はない。

残る点: 暫定解がないとき、前処理後の目的値の定数のずれが分からないとして `best_bound` を -inf で報告する
(mod.rs、既存の方針)。kumeu・momentum1 (4 スレッド) の `bound` が -inf なのはこのため (`root_bound` は出る)。
scripts/run_full_benchmark.py (LP の計測) の `"kOk"` 判定は今回の対象外として残した。

## 確認結果 (修正後、60 秒)

| 問題 | threads | status | 目的値 | 下界 | root_bound | 時間 | 参考: 最適値 (MIPLIB) |
|---|---:|---|---:|---:|---:|---:|---:|
| leo1 | 4 | time_limit | 411943003.36 | 391423765.3 | 390889921.8 | 60.3 | 404227536.16 |
| leo1 | 1 | time_limit | 447104380.64 | 390259817.1 | 389865799.9 | 60.1 | |
| leo2 | 4 | time_limit | 441409651.20 | 388772304.8 | 388103797.7 | 61.9 | 404077441.12 |
| leo2 | 1 | time_limit | 441733465.92 | 388215010.8 | 387668836.1 | 60.4 | |
| neos-3656078-kumeu | 4 | time_limit | - | -inf | -18413.2 | 60.6 | -13172.2 |
| neos-3656078-kumeu | 1 | time_limit | - | -inf | -18413.2 | 60.7 | |
| neos-933966 | 4 | time_limit | 44277 | 318 | 318 | 60.9 | 318 |
| neos-933966 | 1 | time_limit | 49281 | 318 | 318 | 60.6 | |
| momentum1 | 4 | time_limit | - | -inf | 87739.6 | 61.1 | 109143.49 |
| momentum1 | 1 | time_limit | 527462.86 | 85354.6 | 85186.1 | 60.9 | |

5 問とも `time_limit`。目的値は最適値以上、下界は最適値以下で矛盾はない (leo1 の 4 スレッドの暫定解は HiGHS の
429812443.36 より良い)。

### 既定の 40 問 (scripts/run_miplib_benchmark.py、4 スレッド、60 秒、HiGHS の結果は共通)

| | HiGHS | 修正前 (31740cc) | 修正後 |
|---|---:|---:|---:|
| 解けた問題数 (/40) | 20 | 24 | 25 |
| Shifted geomean [s] | 25.07 | 20.45 | 20.22 |
| 目的値の不一致 | | 0 | 0 |

- 解けた問題は減っていない (noswot が 40.7 秒で解けた。時間の揺れの範囲)。目的値の不一致 0
- 30n20b8 は修正後の 1 回目で解なしだったが (修正前 453)、測り直すと修正後 653・修正前は解なしで、並列の揺れ
- dcmulti 1.5 → 4.7 秒、rout 6.9 → 9.0 秒、eil33-2 34.7 → 25.8 秒などの差も 4 スレッドの揺れの範囲と見ている
  (1 回ずつの計測)
