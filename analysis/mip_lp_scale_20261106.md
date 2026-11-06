# MIP の LP (`TwoStageLp`) を尺度を調整した形で持つ試作 `ENOMOTO_MIP_LPSCALE` (2026-11-06)

bench に使う環境変数: base = 既定 (何も付けない)、fix = `ENOMOTO_MIP_LPSCALE=1` (条件のつまみ `ENOMOTO_T_MIP_LPSCALE_MIN_M`・
`ENOMOTO_T_MIP_LPSCALE_MIN_RANGE` は節ごとに書く)。どちらも `ENOMOTO_MIP_THREADS=4` (`--threads 4`)。

(計測の結果は後の節に書く。まず設計。)

## 1. 設計

### 1.1 方針: 尺度つきの「双子」を `TwoStageLp` の中に持ち、元の尺度で仕上げる

`TwoStageLp` (以下「本体」) は今のまま元の尺度の行・境界・費用・標準形を持ち、外から見える値 (x、双対、被約費用、目的値、行の活動量、
基底の状態、DSE の重み、打ち切り値、Farkas の射線など) はすべて本体のものを返す。`ENOMOTO_MIP_LPSCALE=1` で条件に当たれば、作るとき
(`TwoStageLp::new`。再スタートの作り直し・サブ MIP・潜りの LP もここを通る) に行と列の尺度 `r_i`・`c_j` (LP の経路の試作
`ENOMOTO_SIMPLEX_SCALE` と同じ `scale_factors`: 幾何平均の調整を 4 回 + 列の最大を 1、2 の冪、2^-20..2^20) を決め、尺度をかけた
写し (以下「双子」、`tw: Option<Box<TwoStageLp>>`) を本体の中に持つ。

- 双子の係数は `r_i a_ij c_j`、行の境界は `r_i × (lo, up)`、列の境界は `(lo, up) / c_j`、費用は `cost_j c_j`。目的値は尺度によらない
  (打ち切り値・共有する下界はそのまま使える)。
- 本体を変える操作は双子にも尺度をかけて同じ操作をする (尺度は 2 の冪なので境界の変換は丸めがない):
  `set_col_bounds` (`/c_j`)・`set_row_bounds` (`×r_i`)・`set_costs` (`×c_j`)・`add_rows` (新しい行の尺度 `r` は、列の尺度をかけた係数の
  幾何平均 `1/sqrt(min max)` の 2 の冪。列の尺度は変えない)・`delete_rows`・`delete_and_add_rows` (行の尺度も同じ行を消す)。
- 尺度は作るときに決めたまま (行を加えても列の尺度は変えない)。

### 1.2 求解 (`solve_once` の中、`solve_tw`)

1. 双子を本体に合わせる (`tw_pull`、本体の基底が外から置かれたとき (`set_basis`・`set_basis_cols`・`restore_state`・LU の差し替え・
   内点法の頂点の採用・最適面の移動など、`tw_sync` が偽) だけ): 基底 (尺度によらない)、解 `x / c` (非基底列の置き場所の推し量りに使う)、
   打ち切りの続きの置き場所・`ENOMOTO_MIP_BASIS_NB` の希望 (±1、尺度によらない)、DSE 重み (同じ基底なら写すだけ。尺度の違う重みでも
   正しさには関わらない)。双子の LU キャッシュは双子の基底と同じときだけ使われる (双子の中の判定のまま)。
2. 双子を二段解法で解く (`solve_inner`。時間・反復の上限・打ち切り値は同じ。DSE 重み・LU のキャッシュ・打ち切りの続きは双子が自分で持つ)。
3. 結果ごとに、元の尺度で確かめる (HiGHS と同じく「尺度つきで解き、元の尺度での違反を最後に確かめる」):
   - **最適**: 双子の最適基底と `x c` を本体に移し、本体 (元の尺度) で warm start で解き直す (仕上げ。ふつうは 0〜数反復で、LU の
     分解 1 回)。許容誤差の判定は本体の二段解法のもの (元の尺度) なので、返す解は今までと同じ意味で主・双対実行可能。
     本体の LU・DSE 重み・双対・被約費用はこの仕上げで作られる (Gomory の行 `basis_inverse_row`・最適面の移動もそのまま使える)。
   - **打ち切り (`ObjectiveBound`)**: 双子の双対 `y'` を `y = r y'` で元の尺度に戻し、本体の標準形で下界 `L(y) = b·y + Σ min (c - A^T y)_j x_j`
     (`lower_bound_from_basis` と同じ式・同じ小さい値の扱い) を計算し直し、打ち切り値以上なら採る (双対は双対証明に使われる)。
     足りなければ本体で最初から (双子に渡す前の基底から) 解き直す。
   - **実行不能**: 双子の射線を `r` で元に戻し、本体の標準形で Farkas の条件 (`y·b` が `(A^T y)·x` の箱の上の範囲の外) を確かめて採る。
     確かめられなければ本体で解き直す。
   - **非有界・数値の破綻**: 本体で解き直す (切片問題の境界は標準形の空間の値なので、双子のものは使わない)。
   - **時間・反復の上限**: そのまま返す (本体の基底・解は変えない。双子は止まったところを持ち、`tw_sync` が真なら次の求解で続きから解く)。
4. 双子で解いた後は `tw_sync` を真にする (次の求解は双子の基底から始まる。本体の仕上げが基底を動かしても双子は自分の最適基底から)。

### 1.3 状態の保存と兄弟の LU・DSE 重み

- `save_state`・`restore_state` (強分岐・潜り): `TwoStageState` に双子の基底・解・LU・DSE 重み・`tw_sync` を加える。戻すと双子もその時の
  状態に戻る (列の境界は本体の `set_col_bounds` が双子にも写す)。
- `lu_snapshot`・`adopt_lu_snapshot` (兄弟ノードの LU): `LuSnapshot` に双子の LU を加え、戻すときは双子にも戻す。
- `dse_snapshot`・`dse_age`・`recompute_dse_exact`: 双子が本体と同じ基底を持つ (`tw_sync` かつ基底が同じ) なら双子の重みを使う
  (保存した重みはこの LP (と写し) にしか戻らない。重みは正しさに関わらない)。

### 1.4 影響する関数 (mip_lp.rs)

`new` (双子を作る)・`new_raw` (新設: 双子を作らない。写し `solve_scaled`・`ipm_scale` はこちら)・`set_col_bounds`・`set_row_bounds`・`set_costs`・
`add_rows(_inner)`・`delete_rows(_inner)`・`delete_and_add_rows`・`set_basis`・`set_basis_nb`・`set_basis_cols`・`adopt_lu_snapshot`・
`lu_snapshot`・`save_state`・`restore_state`・`dse_snapshot`・`dse_age`・`recompute_dse_exact`・`solve_once` (双子へ)・`solve_inner0`
(本体で解いたら `tw_sync` を偽)・`clear_memo` (`tw_sync` を偽。行の操作は双子にも写すので戻す)。
内点法 (`interior_point`・競争の内点法側・`solve_ipm_rescue`)・診断・書き出しは本体の (元の尺度の) 標準形のまま。

### 1.5 条件

`ENOMOTO_MIP_LPSCALE=1` のとき、行の数 `>= ENOMOTO_T_MIP_LPSCALE_MIN_M` (既定 0) かつ係数の幅 `>= ENOMOTO_T_MIP_LPSCALE_MIN_RANGE`
(既定 0) の LP だけ双子を持つ。尺度がすべて 1 なら持たない。条件は作るときの行で決める (サブ MIP・再スタートの LP は作るたびに判定)。

### 1.6 費用 (推測)

本体の仕上げで 1 回ごとに LU の分解が 1 回増える (双子の分解と合わせて 2 回)。行の追加・境界の変更も 2 倍。重い LP (momentum1 のノードの
1 回 5,125 反復) では小さいが、軽い LP の多い問題では害になりうる → 条件で絞る。
