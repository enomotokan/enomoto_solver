# Local-MIP の本家のコードと ENOMOTO の実装 (`src/mip/localmip.rs`) の比べ (2026-11-05)

- 目的: `src/mip/localmip.rs` は「論文は手元にないので、覚えている範囲の手順で組んだ」もの。本家の公開コードと突き合わせ、
  同じ所・違う所・ENOMOTO にない所を並べ、足すと良さそうなものに優先度をつける。**コードは変えていない** (読むだけの調査)。
- 本家: <https://github.com/shaowei-cai-group/Local-MIP>
  - 最新 (main): コミット **3bd2d611bf9f2e1a8c759a1b26c0385cd8aaf94e** (2026-08-12、"Version 2.0.10 (#17)")。以下「v2」。
  - 論文の実験に使った版: タグ **v1.0.1** (コミット f1422ca8c2b0964c8a787f81b71cce1ef116e3c5)。README に「論文 (CP 2024・Artificial
    Intelligence 2025) の結果は Local-MIP 1.0 で得た」とある。以下「v1」。論文の手順に近いのはこちら。
  - ライセンス: **MIT** (v2 の `LICENSE`、"Copyright (c) 2025 Local-MIP")。v1 にも `LICENSE` がある。
  - 取得したものはスクラッチの場所に置き、ビルド・実行はしていない (v1 には `bin/Local-MIP` の実行形式も入っているが触っていない)。
    本家のコードはこのリポジトリに入れていない。
- 論文: Lin・Zou・Cai, CP 2024 "An Efficient Local Search Solver for Mixed Integer Programming" (LIPIcs 307, 19:1–19:19。依頼文の題とは
  少し違う。本家の README の表記に従う)。拡張版: Lin・Cai・Zou・Lin, Artificial Intelligence 348 (2025) 104405 "Local-MIP: Efficient
  local search for mixed integer programming"。**論文の本文は読んでいない** (取得していない)。手順は本家のコードから読み取った。
  論文に書かれた手順の名 (tight move・lift move・breakthrough move・mixed tight move・robust bonus など) とコードの関数の対応は
  コードの名前とコメントからの推測を含む。

## 1. 本家のコードの構成

### v2 (main、C++20、`src/` 約 1.1 万行。探索の本体は 2,000 行ほど)

| ファイル | 中身 |
|---|---|
| `src/local_search/Local_Search.cpp` `run_search` | 主ループ。違反なしなら `update_best_solution` (目的が breakthrough していれば) → `lift_move`。違反ありか lift が失敗したら `explore_neighbor` → `apply_move` |
| `Local_Search.cpp` `explore_neighbor` | 近傍の並びを順に評価し、正のスコアの手が出たらそこで止める。**最後の近傍 (random) の前に重みを更新** |
| `Local_Search.cpp` `reset_default_neighbor_list` | 既定の近傍の並び: `unsat_mtm_bm` → `sat_mtm` → `flip` → `easy` → `unsat_mtm_bm_random` |
| `src/local_search/neighbor/neighbor.cpp` | `inequality_mixed_tight_operation`・`equality_mixed_tight_operation`・`breakthrough_operation` (1 列の手の量) |
| `neighbor/explore_unsat.cpp` | 違反行からの mixed tight move + breakthrough move (目的の全部の列) |
| `neighbor/explore_sat.cpp` | 満たされた行からの mixed tight move (実行可能解を見つけた後だけ) |
| `neighbor/explore_flip.cpp` | 0-1 列の反転 (既定は 0 手 = 使わない) |
| `neighbor/explore_easy.cpp` | 列を 0・境界・中点へ動かす手 (v2 で足されたもの) |
| `neighbor/explore_unsat_random.cpp` | 違反行 1 本 + breakthrough からの手 (random walk に当たる) |
| `neighbor/neighbor.h` | BMS (`sample_idxs`・`sample_op`)、タブー (`tabu`・`tabu_latest`) |
| `src/local_search/scoring/neighbor_scoring.cpp` `progress` | 違反のある間のスコア (2 段: スコア + bonus、同点は age) |
| `scoring/lift_scoring.cpp` `lift_age` | lift move のスコア (目的の改善量、同点は age) |
| `src/local_search/lift_move/lift_move.cpp` | lift move (最良の 1 手、幅の差分更新) |
| `src/local_search/weight/weight.cpp` | 重みの更新 (`monotone` 既定・`smooth`) |
| `src/local_search/restart/restart.cpp` | 再出発 (`best` 既定・`random`・`hybrid`) |
| `src/local_search/start/start.cpp` | 初期解 (`zero` 既定・`random`・`objective`・`locks`) |
| `src/model_data/Model_Manager.cpp` `process_after_read` | 前処理: 等式を 2 本の不等式に分ける、`>=` を `<=` にそろえる、1 列の行で境界を締める、固定列を消して伝播 |
| `src/utils/paras.h` | パラメータと既定値 |
| `example/parallel-multiseed/` | 1 つの問題を共有して種を変えた探索を呼び出し側のスレッドで並べる例 (本体は 1 スレッド) |

### v1 (論文の版、`code/LocalSearch/` 約 1,200 行)

`LocalMIP.cpp` (主ループ・`ApplyMove`・`Restart`・`Allocate` に既定値)、`UnsatTightMove.cpp`、`SatTightMove.cpp`、`FlipMove.cpp`、
`RandomTightMove.cpp`、`TightOperator.cpp` (`TightScore`・`TightDelta`・`UpdateWeight`・`SmoothWeight`)、`LiftMove.cpp`
(`LiftMoveWithoutBreak`)。前処理は `code/ReaderMPS.cpp` (`TightenBound`・`TightBoundGlobally`)。

v1 の主ループ (`LocalMIP::LocalSearch`):

```
違反なし → (初回か目的が暫定解より良ければ) 暫定解を更新、目的の行の右辺 = 暫定解 - OptimalTol
          → LiftMoveWithoutBreak が成功したら次へ
UnsatTightMove (違反行 + breakthrough) が正の手を出せなければ
  → SatTightMove (実行可能解を見つけた後だけ) → FlipMove
  → それでもなければ 確率 (1 - 約 4/10000) で UpdateWeight、それ以外 SmoothWeight → RandomTightMove
```

v2 の `run_search` + `explore_neighbor` も同じ形 (近傍の並びを順に試し、どれも正でなければ重みを更新して random)。

## 2. 観点ごとの比べ

記号: 「同」= ほぼ同じ、「違」= 違う、「無」= ENOMOTO にない。ENOMOTO の場所はすべて `src/mip/localmip.rs`。

### 2.1 手の種類

| 手 | 本家 | ENOMOTO | |
|---|---|---|---|
| 違反行からの tight move | v2 `explore_unsat_mtm_bm` + `inequality_mixed_tight_operation` (`neighbor.cpp`)。行をちょうど満たす量 `-(gap/coeff)`、整数は floor/ceil で満たす側へ。**境界を越えるなら満たす向きの境界まで** (違反行なら係数 > 0 で下限)。v1 `UnsatTightMove` + `TightDelta` も同じ | `St::tight_value` (182 行) + `LocalMip::run` の BMS (520〜541 行)。行をちょうど満たす値、整数は満たす向きに丸め、境界で clamp | 同 |
| 満たされた行からの tight move | v2 `explore_sat_mtm`、v1 `SatTightMove`。満たされた不等式の列を、その行の余裕を使い切る所まで動かす (境界を越えるなら**反対の向き**の境界、`inequality_mixed_tight_operation` の else 側)。**実行可能解を見つけた後だけ**。`is_inferred_sat` (前処理で常に満たされると分かった行) と等式は除く | なし | **無** |
| breakthrough move | v2 `breakthrough_operation` を `explore_unsat_mtm_bm`・`explore_unsat_random_bm` の中で**目的の全部の列について**候補に足す (実行可能解があり、今の目的が暫定解 - opt_tol を下回っていない間)。v1 も同じ (`UnsatTightMove` の後半で目的の行 `localObj` に `TightDelta`) | 目的の行 (index `m`) を「違反行」の集合 `vset` に入れ、違反行の抜き取り (3 本) で選ばれたときだけ、その行の列 (最大 64 本) を見る | 違 (候補の入り方が確率的で少ない) |
| mixed tight move の連続列 | 連続列は丸めない (`is_real()`)。整数列だけ floor/ceil (等式の片側は v2 `equality_mixed_tight_operation` で round。ただし既定は等式を分けるのでこちらは使われない) | 同じ (`p.is_int[j]` のときだけ丸め) | 同 |
| lift move | v2 `lift_move`・`lift_move_operation`、v1 `LiftMoveWithoutBreak`。目的の各列について「全部の行を満たしたまま動ける幅」を求め、**目的を最も良くする 1 手だけ**を当てる (同点は age)。幅は差分更新 (動かした列と行を共有する列だけ計算し直す、`m_feas_touch_vars`、`m_is_keep_feas`) | `St::lift_sweep` (273 行)。費用のある列を乱数の順に見て、良くなる手をその場で全部当てる掃引を、動かなくなるまで繰り返す (`while s.lift_sweep(..) > 0 {}`、446 行)。毎回全部計算し直す | 違 (1 手か掃引か。掃引は貪欲に 1 回で行き着くので、本家より lift の段が短い代わりに、1 回の手間が目的の列の非零の数) |
| 0-1 の反転 | v1 `FlipMove` (0-1 列を 20 本抜き取り)。v2 `explore_flip` は既定 0 手で**使わない** | なし | 無 (v2 も既定で使わない) |
| easy move | v2 だけ `explore_easy`: 固定でない列を 5 本抜き取り、0・下限・上限・(連続で両側有限なら) 中点へ動かす手 | なし | 無 |
| random walk | v2 `explore_unsat_random_bm`、v1 `RandomTightMove`: 違反行 1 本の全部の列 + breakthrough の手から BMS (v2 250、v1 150) 本、**負でも最良**を当てる。タブーは「直前の手の逆」だけ禁じる (`tabu_latest`) | 違反行 1 本 (目的の行も含む) の列を最大 64 本、最良を当てる。タブーは見ない (633〜644 行) | ほぼ同 (量が少ない) |
| 組の手・連鎖の手 | なし | `ENOMOTO_MIP_LOCALMIP_PAIR` (組)、`chain_move` (827 行、ejection chain。FJ と Local-MIP を混ぜる既定の形で使う) | ENOMOTO にだけある |

### 2.2 手の選び方

| 項目 | 本家 | ENOMOTO | |
|---|---|---|---|
| スコア (制約) | v2 `Scoring::progress`: 不等式で 違反 → 満たす = **+2w**、満たす → 違反 = −2w、違反のまま 減る = **+w**・増える = −w (向きで数える)。v1 `TightScore`: それぞれ +w・−w・**+w/2**・−w/2 | `St::score` (196 行): 違反量が減れば +w、増えれば −w。**「満たした」と「減っただけ」を区別しない** | 違 |
| スコア (目的) | 実行可能解を見つけた後は、目的に効く列の手で**目的が下がれば +2w0、上がれば −2w0** (v2。v1 は ±w0)。目的の行の「違反」かどうかに関わらず数える | 目的の行 `c x <= obj_up` の**違反量**の増減で数える。目的の行が満たされている間 (暫定解より良い所にいる間) は目的が悪くなっても 0 | 違 |
| 同点の崩し方 | v2 `progress_bonus` (既定): bonus = 「動かした後の目的が暫定解より良い」なら +w0 (breakthrough bonus)、それでも同点なら **age が古い列** (最後に動かした手数が小さい)。v1: subscore = breakthrough bonus + **robust bonus** (行が「余裕 > 許容誤差」になれば +w、なくなれば −w)、age は見ない | `breakthrough` (236 行) の真偽だけ。それ以外の同点は先に見た手 | 違 (robust bonus・age なし) |
| BMS のサンプル数 | v2: 違反行 **10** 本 (`bms_unsat_con`)、その行の**全部の列** + breakthrough の手を集めてから **2250** 手 (`bms_unsat_ops`) を重複なしに抜き取る。満たされた行は 1 本・80 手、easy 5 列、random 250 手。v1: 違反行 12 本、手は実行可能解の前 2000・後 3000、満たされた行 20 本・190 手、flip 20、random 150 | 違反行 **3** 本 (`ENOMOTO_T_MIP_LOCALMIP_SAMPLE_ROWS`)、各行 **64** 列 (`..._SAMPLE_COLS`)。行も列も**重複ありの抜き取り** (`s.below(..)`)。1 手で見る候補は最大 192 | 違 (候補が 1 桁少ない) |
| 0-1 列の重複 | 1 手の中で同じ 0-1 列は 1 回だけ評価 (`m_binary_op_stamp`、v1 `scoreTable`) | なし (同じ手を何度も評価しうる) | 無 |
| タブー | 増やしたら `tabu_base + rand % tabu_var` 手のあいだ減らさない (逆も)。v2 は 4 + [0,7)、v1 は 3 + [0,10) | 3 + [0,10) (`St::apply`、246 行) | 同 (v1 と同じ値) |
| age | 列ごとに最後に増やした・減らした手数 (`m_var_last_inc_step`・`m_var_last_dec_step`)。同点崩し (v2) と lift (v1 は動かす向きの手数) で使う | 持っていない | 無 |

### 2.3 重み・random walk・再出発

| 項目 | 本家 | ENOMOTO | |
|---|---|---|---|
| 重みの更新 | v2 既定 `monotone` (`weight.cpp`): 違反行の重みを +1、**違反行がなく実行可能解があるときだけ**目的の重みを +1。平滑化なし (`smooth` を選べば確率 `smooth_prob`/10000 = 既定 1/10000 で、満たされた行の重みを −1 (0 まで)、目的は breakthrough していれば −1)。v1: 確率 ≒ 4/10000 で `SmoothWeight` (満たされた行を −1、0 まで)、それ以外 `UpdateWeight` (v2 monotone と同じ) | 確率 `sp` = 0.0003 (`ENOMOTO_T_MIP_LOCALMIP_SP`) で満たされた行を −1 (**1 まで**)、それ以外は違反行 (**目的の行を含む**) を +1 (612〜625 行) | ほぼ同 (v1 に近い)。目的の重みの上げ方が違う: ENOMOTO は目的の行が違反していればいつも上げるので、制約の違反が残る間も目的の重みが育つ |
| 重みの上限 | v1 は `weightUpperBound` = 1e7 を持つが使っていない。v2 はなし | なし (f64) | 同 |
| 目的の行の右辺 | 暫定解の値 − `opt_tolerance` (1e-4、絶対) (`update_best_solution`) | 暫定解の値 − `ls_obj_step` (費用が全部整数なら 1、そうでなければ相対 1e-6 程度、`heuristics.rs` 1202 行) | 違 (ENOMOTO の方が整数目的で賢い) |
| 再出発 | v2: 暫定解が**最後に良くなってから 100 万手** (`restart_step`) で、`best` (暫定解から、なければ乱数) に戻し、重みを 1 に、タブー・age を 0 に。v1: `Restart()` (半々で暫定解と乱数) は書かれているが**呼ばれていない** (再出発なし) | 「この回の違反行の最少」が 30 万手 (`ENOMOTO_T_MIP_LOCALMIP_RESTART`) 減らなければ、暫定解 (なければ出発点の整数列の 1 割を乱数に) から、重み・タブーを戻す (498〜519 行) | 違 (数え方と既定の手数) |

### 2.4 初期解・前処理・列と行の扱い

| 項目 | 本家 | ENOMOTO | |
|---|---|---|---|
| 初期解 | v2 既定 `zero` (0 を境界に収めた値)。ほかに `random`・`objective` (目的の向きの有限な境界)・`locks` (ロックの少ない向き)。v1 も 0 を境界に収めた値 | 各列の下限 (なければ上限、なければ 0) (`LocalMip::new`、354 行)。`set_start`・`inject` で外から与えられる | ほぼ同 (下限 < 0 < 上限の列だけ違う) |
| 前処理 | v2 `Model_Manager::process_after_read`: 等式を `<=` 2 本に分ける (`split_eq` 既定 1)、`>=` を `<=` に、1 列の行で境界を締めてその行を消す (`singleton_deduction`)、固定列を消して 1 列・0 列になった行を連鎖して処理 (`global_propagation`)。**連続列のない問題だけ** (`bound_strengthen` 既定 1 = IP のみ、2 で MIP も)。活動度による境界の締め込みはしない。v1 `ReaderMPS::TightenBound`・`TightBoundGlobally` も同じ程度 | ENOMOTO 本体の前処理の後の問題 (FJ_P3 の前処理をかけ直したものも) を受け取る。本家よりずっと強い | ENOMOTO の方が強い (足す必要なし) |
| 等式 | 既定は 2 本の不等式に分けて扱う (違反するのは片方だけ。満たされた側は `sat_mtm` の対象になる) | 行の両側 `[row_lo, row_up]` をそのまま持つ (`St::bounds`・`viol_at`) | 違 (分ける利点は「満たされた行からの手」と組んだときに出る。ENOMOTO は分けずに同じことができる) |
| 一般整数 | 丸めは tight move と同じ。random 出発・再出発で範囲内の一様乱数 | 丸めは同じ | 同 |
| 連続列 | 丸めない。活動度の誤差対策 (下記) | 丸めない | 同 |

### 2.5 データ構造・計算量

| 項目 | 本家 | ENOMOTO | |
|---|---|---|---|
| 違反行の集合 | 配列 + 位置の表 (`m_con_unsat_idxs`・`m_con_pos_in_unsat_idxs`)、v2 は満たされた行の集合も持つ (`m_con_sat_idxs`、`sat_mtm` の抜き取り用) | 配列 + 位置の表 (`vset`・`vpos`)。満たされた行の集合はない | 同 (満たされた側は無) |
| 行の左辺 | 差分更新 (v2 `update_affected_activities`)。係数・境界がすべて整数で桁が収まれば double で厳密 (`can_use_exact_double_activity`)、そうでなければ long double で、触った行を 10 万手ごとに数え直す (`activity_period`)。**v1 の `ApplyMove` は触った行を毎回全部数え直す** (行の長さに比例) | 差分更新、10 万手ごとに全行を数え直す (647 行) | 同 (v2 と) |
| スコア | 差分更新はしない。候補ごとに列の非零を全部回る (O(列の非零)) | 同じ (O(列の非零)) | 同 |
| lift の幅 | 目的の列ごとの幅を配列に持ち、動かした列と行を共有する列だけ計算し直す | 毎回全部計算し直す (掃引の回数 × 目的の列の非零) | 違 |
| 時間の確かめ | 別スレッドの時計が `m_terminated` を立てる (主ループは atomic を読むだけ) | 16 手ごとに `Instant::now()` | 違 (軽微) |
| スレッド | 1 本の探索。`Prepared_Model` を共有して種を変えた探索を呼び出し側で並べられる (`example/parallel-multiseed/`) | 呼び出し側の割り振り (3 節) | — |

### 2.6 パラメータの既定値 (まとめ)

| | v1 (論文) | v2 (main) | ENOMOTO |
|---|---|---|---|
| 違反行の抜き取り | 12 | 10 | 3 |
| 違反行の手の抜き取り | 2000 (解の前) / 3000 (後) | 2250 | 3 × 64 (重複あり) |
| 満たされた行 / 手 | 20 / 190 | 1 / 80 | なし |
| flip | 20 | 0 | なし |
| easy | なし | 5 | なし |
| random の手 | 150 | 250 | 64 |
| タブー | 3 + [0,10) | 4 + [0,7) | 3 + [0,10) |
| 平滑化の確率 | 約 4/10000 | 1/10000 (`smooth` のとき。既定 `monotone` は平滑化なし) | 3/10000 |
| 再出発 | なし | 改善なし 100 万手 | 違反行の最少が 30 万手減らない |
| 目的の刻み | 1e-4 (絶対) | 1e-4 (絶対、`opt_tolerance`) | `ls_obj_step` |
| 乱数の種 | 2832 固定 | 0 (= 既定の種) | 呼び出し側 |

## 3. ENOMOTO での Local-MIP の使われ方 (呼び出し元)

- **既定で動くもの**:
  - `ENOMOTO_MIP_FJ_LMIP_MIX` (既定で使う、`ENOMOTO_MIP_NO_FJ_LMIP_MIX` で切る、`solver.rs` `fj_lmip_mix_on`): 主の求解が起こす
    **1 本の FJ のスレッド**の中で、FJ2 (1 回単位) と Local-MIP + 連鎖 (`LocalMip::run` を `ENOMOTO_T_MIP_FJ_MIX_SLICE` = 0.2 秒ずつ続きから)
    を時間で分け合う (`heuristics.rs` `fj_lmip_mix`)。解がない間は半々、解が出たら最良の解を出した方に 0.75 (`ENOMOTO_T_MIP_FJ_MIX_LEAD`)。
    FJ の解が良ければ `LocalMip::inject` で Local-MIP に渡して磨かせる。時間は `ENOMOTO_T_MIP_LOCALMIP_TIME_FRAC` (0.1) × 時間上限
    (最大 10 秒) と FJ の上限の大きい方、解がない間は `ENOMOTO_MIP_FJ_NOSOL_EXTEND` で延ばす。問題は FJ_P3 の前処理をかけ直したもの。
    目的が全部 0 の問題では使わない (FJ だけ)。
  - `ENOMOTO_MIP_FEAS_FJ_EARLY=fl` (既定、`mod.rs` `feas_fj_lmip_early`): 前処理後の目的が全部 0 の問題で、LP による固定の前に、
    スレッドを **FJ2 3 対 Local-MIP 1** (4 分の 1 の切り捨てで少なくとも 1 本。4 本で 3 + 1、8 本で 6 + 2) に分けて並べ、
    どれかが見つけたら最適で終える。上限は 0.1 × 時間上限 (`ENOMOTO_T_MIP_FEAS_FJ_EARLY_FRAC`) と前処理の期限の早い方。
    スレッドが 1 本なら交互 (`ENOMOTO_T_MIP_FEAS_EARLY_LMIP_SLICE` 0.2 秒)。
- **試作 (既定では使わない)**: `ENOMOTO_MIP_LOCALMIP=1/2` (別スレッド / FJ の代わり)、`ENOMOTO_MIP_FJ_LOCALMIP` (FJ2 が 2 回続けて
  解なしなら切り替え、`fj_search_fb`)、`ENOMOTO_MIP_ROOT_LMIP` (根の暫定解から磨く)、`ENOMOTO_MIP_ROLE_LMIP` (ヒューリスティクス役で
  ALNS と競う)、`ENOMOTO_MIP_LOCALMIP_CHAIN`・`_PAIR`、診断 `ENOMOTO_MIP_DEBUG_FJ_VS_LMIP`・`ENOMOTO_MIP_DEBUG_LMIP_FROM_LP`。
- `localmip.rs` 冒頭の「既定では使わない」は、混ぜる形と早めの探索が既定になった今は古い (コメントの直しは別の作業)。

## 4. 評価の比べ

- 本家の README (v2) は数値の評価を載せていない (論文を参照)。v1 には `result/result.csv` (555 問 × 7 ソルバー × 10・60・300 秒の
  目的値) と問題の一覧 `benchmark/*.txt` (MIPLIB 2017 の部分集合 351 問 + BPP 60・JSP 80・OSP 60) がある。
  この csv から実行可能解を見つけた問題の数を数えた (筆者の数え。値の良し悪しは見ていない):

  | 秒 | Local-MIP | Feasibility Jump | Gurobi (heur) | CPLEX | SCIP | HiGHS |
  |---|---|---|---|---|---|---|
  | 555 問、10 秒 | **364** | 185 | 331 | 326 | 271 | 158 |
  | 555 問、300 秒 | **429** | 206 | 419 | 400 | 385 | 331 |
  | MIPLIB の部分 354 行、10 秒 | 198 | 124 | 216 | 206 | 140 | 79 |
  | MIPLIB の部分 354 行、300 秒 | 238 | 145 | 264 | 238 | 214 | 174 |

  - MIPLIB の部分では Local-MIP は FJ の 1.6 倍ほどの問題で解を出し、商用ソルバーに近い。BPP・JSP・OSP (等式・一般整数の多い組合せ
    最適化の問題) で差が大きい。README は MIPLIB の open な問題 10 問で新記録 (sorrell7・genus-sym-g31-8・supportcase22 など) を挙げる。
  - 論文本文の主張 (比べ方・新記録の数など) は確かめていない。
- ENOMOTO の記録にある効き目 (要点):
  - `analysis/heur_ls_20261023.md` 追記 2 (55 問、FJ_P3 の問題で FJ と Local-MIP + 連鎖を 3 秒ずつ単独): **Local-MIP が良い 39 問・
    FJ が良い 13 問**。Local-MIP だけが解を出す 8 問 (30n20b8・beasleyC3・beavma・blend2・flugpl・mc11・neos-1456979・timtab1)、
    FJ だけが解を出す 4 問 (h80x6320d・hypothyroid-k1・nursesched-sprint02・nw04。Local-MIP は違反 3〜124 行で止まる)。
    目的値の差が大きい例: gen-ip054 22988 → 6932、mzzv42z −510 → −8978、mik-250-20-75-4 0 → −30407。
    連鎖なしの Local-MIP は 30n20b8・physiciansched6-2 で FJ と同じく違反 1 行で止まった (連鎖の手が効いている)。
  - 同 追記 3: 混ぜる形 (mix) は 56 問の主ギャップの和を 2.4〜3.0 → 1.5〜2.0、最初の解の時刻の幾何平均を 2.3 → 1.4 秒に。
  - `analysis/feas_early_lmip_20261102.md`: 目的 0 の 9 問で、Local-MIP が fhnw-binpack4-48 (FJ は 60 秒で解なし → 1.5 秒)・
    neos-3004026-krka (17 秒 → 1.3 秒) を解く。ns1116954 は Local-MIP では違反 1 行で止まり FJ が要る。
  - 連鎖の手は 1 手が重い (ns1116954 で連鎖ありは連鎖なしより 1 手が 200 倍重い、roll3000 は 9 千手 vs 60 万手)。

## 5. ENOMOTO に足すと良さそうなもの (優先度順)

推測を含む。効き目はどれも未計測。

1. **【高】候補の数を本家並みに広げ、0-1 列の重複を除く** (違反行 3 → 10 本程度、行の列は全部集めてから 2000 手程度を重複なしに抜く)。
   - 本家: `neighbor/explore_unsat.cpp` `explore_unsat_mtm_bm`、`neighbor.h` `sample_idxs`・`sample_op`、`scoring/neighbor_scoring.cpp`
     の `m_binary_op_stamp`、既定値 `utils/paras.h` (`bms_unsat_con` 10・`bms_unsat_ops` 2250)。
   - ENOMOTO: `LocalMip::run` の BMS (520〜541 行)、`ENOMOTO_T_MIP_LOCALMIP_SAMPLE_ROWS`・`_SAMPLE_COLS` の既定、重複除きの印を
     `St` に足す。
   - 理由 (推測): 今は 1 手で最大 192 候補・重複あり。正の手を見落として重み更新 + random に落ちる回が多いと思われる。連鎖の手は
     「正の手がないとき」に呼ばれるので、見落としが重い連鎖の呼び出しを増やしている可能性もある。手が重くなる分は手の数で測る。
2. **【高】breakthrough move を目的の全部の列から毎回候補に入れる** (目的の行を違反行の抜き取りに任せない)。
   - 本家: `neighbor.cpp` `breakthrough_operation`、`explore_unsat.cpp` の後半 (`m_is_found_feasible && !m_current_obj_breakthrough`)。
   - ENOMOTO: `run` の BMS で、`s.obj_up` が有限で目的の行が違反しているとき `obj_cols` の手を (抜き取り込みで) 必ず足す。
   - 理由 (推測): 違反行が多いと目的の行が 3 本の抜き取りに入る確率が低く、暫定解の後の改善が遅い。
3. **【高】スコアを本家の 2 段に合わせる** (満たす遷移 2w・違反のまま減る w。目的は暫定解の後いつも ±2w0。同点は breakthrough bonus → age)。
   - 本家: `scoring/neighbor_scoring.cpp` `Scoring::progress` (v2)、v1 `TightOperator.cpp` `TightScore` (robust bonus つき)。
   - ENOMOTO: `St::score`・`St::score2`・`St::breakthrough`、`chain_move` の `contrib` (Local-MIP 側の向きの数え)、
     age のための「最後に動かした手数」を `St::apply` で記録。
   - 理由 (推測): 今は (a) 満たす手と減らすだけの手が同じ点で、(b) 目的の行を満たしている間は目的を悪くする手を罰しないので、
     暫定解の後に目的を大きく戻す手を選びやすく、(c) 同点が先着順で、同じ列を回しがち。
4. **【中】満たされた行からの tight move (sat_mtm) を足す** (実行可能解の後だけ、正の手がないときの 2 段目)。
   - 本家: `neighbor/explore_sat.cpp` `explore_sat_mtm`、`inequality_mixed_tight_operation` の満たされた側 (反対の境界)、
     `Local_Search.cpp` `m_con_sat_idxs` (満たされた行の集合)、`is_inferred_sat` の除外。
   - ENOMOTO: `St` に満たされた行の抜き取り (行を乱数で選んで満たされていれば使う、v1 の `SatTightMove` 方式なら集合は要らない)、
     `tight_value` に「満たされた行の余裕を使い切る値」を足す。ENOMOTO は両側の行なので、余裕の近い側へ。
   - 理由 (推測): 暫定解の後、目的の行だけが違反している局面で、他の行の余裕を使う手が今は候補に入らない (目的の行の列の
     tight move しかない)。冒頭のコメントでも「入れていない」とされている。
5. **【中】lift move を「最良の 1 手 + 幅の差分更新」にするか、掃引の順を目的の改善量順にする**。
   - 本家: `lift_move/lift_move.cpp` `lift_move`・`lift_move_operation`、`scoring/lift_scoring.cpp` `lift_age`。
   - ENOMOTO: `St::lift_sweep`、`run` の 446 行。
   - 理由 (推測): 乱数の順の貪欲な掃引は、先に動いた列が後の大きな改善の幅を食う。一方 ENOMOTO の掃引は 1 回で終わるので速い。
     どちらが良いかは問題による。まず mzzv42z・gen-ip054 など lift の効く問題で、掃引 1 回あたりの改善と手間を測ってから。
6. **【中】目的の重みの上げ方を本家に合わせる** (違反行が目的の行だけのときだけ目的の重みを上げる)。
   - 本家: `weight/weight.cpp` `monotone_update` (`m_con_unsat_idxs.empty()` のときだけ `m_con_weight[0]++`)。
   - ENOMOTO: `run` の重み更新 (620〜625 行) で `r == m` を、`m` 以外の違反行があるときは上げない。
   - 理由 (推測): 今は制約の違反が残る間も目的の重みが育ち、実行可能に戻る力が弱まる。3 と組で直す (3 で目的のスコアを
     いつも数えるなら、重みの意味が変わる)。
7. **【低】easy move (0・境界・中点) と 0-1 反転** を random の前に試す。
   - 本家: `neighbor/explore_easy.cpp`、`neighbor/explore_flip.cpp` (v2 は既定で反転 0 手)、v1 `FlipMove.cpp`。
   - ENOMOTO: `run` の「正の手がない」分岐 (612 行) の前に近傍を 1 段足す。
   - 理由: v2 が easy を足しているので、本家の実験では効いたと思われる (推測)。手間は小さい。
8. **【低】再出発の数え方を「暫定解が最後に良くなってから」にし、既定の手数を見直す**。
   - 本家: `restart/restart.h` `should_restart`、`restart.cpp` `best_restart`。
   - ENOMOTO: `run` の 489〜519 行 (`round_best`・`since_improve`)。混ぜる形では Local-MIP は 0.2 秒ずつしか回らず、
     30 万手に届くことは少ないと思われるので、効き目は小さい (推測)。
9. **【低】random walk でもタブー (直前の手の逆だけ) を見る、random の候補を増やす (64 → 150〜250)**。
   - 本家: `neighbor/explore_unsat_random.cpp`、`neighbor.h` `tabu_latest`。ENOMOTO: `run` の 626〜644 行。

足す必要のないもの: 本家の前処理 (ENOMOTO 本体の前処理の方が強い)、等式の分割 (両側の行のままで同じことができる)、
活動度の厳密 double の判定 (ENOMOTO は 10 万手ごとの数え直しで足りている。誤差で止まる例の記録はない)。

### 確かめ方の案

- 診断 `ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=3` (55 問、heur_ls 追記 2 の形) で、上の 1〜3 を 1 つずつ入れた Local-MIP を今のものと比べる
  (違反行の最少・最初の解・3 秒の最良の値・手の数)。FJ だけが解く 4 問 (h80x6320d・hypothyroid-k1・nursesched-sprint02・nw04) と
  Local-MIP が違反 1 行で止まる問題 (ns1116954・lectsched-1・roll3000) が変わるかを見る。
- 次に既定の混ぜる形 (`fj_lmip_mix`) と早めの探索 (`feas_fj_lmip_early`) の上で、40 問 + 16 問・目的 0 の 9 問を測る。
