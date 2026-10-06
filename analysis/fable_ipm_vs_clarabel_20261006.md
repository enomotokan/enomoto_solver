# 内点法の Clarabel との比較: 遅さの原因と高速化の実装策 (2026-10-06)

対象: `src/interior_point/boxed.rs` (PIQP 型の近接内点法 IP-PMM、Newton 系は `src/interior_point/kkt.rs` の正規方程式
Cholesky (faer、AMD、既定は逐次分解) または拡大系 LDLᵀ) と、Clarabel 0.11.1 (pip の `clarabel`、Rust 製。等質自己双対埋め込み、
拡大系 (準定値 KKT) の LDLᵀ を QDLDL または faer で分解) の比較。ブランチ `claude/confident-faraday-0cj5gq` (HEAD `338c0b4`)、
**src は変更していない**。試験的な切り替えと問題の書き出しは別の worktree (スクラッチ、下記) で行った。
計測は 4 コア (Intel Xeon 2.80 GHz) / 15 GB の計算機。時間の計測は `benchmarks/crossover/ab14/` の比較が終わってから行い、
反復数だけを見る調査 (CPU 1 スレッド、nice 19) は並行して行った。

## 0. 要約

- **enomoto の内点法は Clarabel より遅い。原因の大半は「反復数」ではなく、反復数と 1 反復の手間の両方で、しかも
  性質の違う 2 つの要因に分かれる。**
  1. **反復数**: 既定 (Netlib の小さな 60 問、§2.1) で enomoto は Clarabel の **1.14 倍** の反復を使い、perold・pilot4 は 200 反復で
     収束しない。原因は正則化 (近接項) の初期値と下限 (§3): `RHO0 = DELTA0 = 0.1` は PIQP の既定 (`rho_init = 1e-6`,
     `delta_init = 1e-4`) より 3〜5 桁大きく、序盤〜中盤で Newton 方向を歪めて中心性を失わせ (ship04s: 7 反復目で
     `min(sz)/μ = 7e-3`、歩幅 0.05〜0.2 が 30 反復続く)、一方で下限 `1e-10` (PIQP は収束間際に `1e-13` へ下げる) は終盤に
     残差の床 `δ‖λ − y‖ ≈ 1e-9` を作って、目的値で測るギャップが `1e-7` で止まる (stair、bnl1)。
     **初期値 1e-4・下限 1e-13 に替えるだけで 60 問すべて収束し反復数は 0.79 倍 (Clarabel の 0.94 倍)、Gondzio の補正子 2 回を
     加えると 0.69 倍 (Clarabel の 0.82 倍)** になる (§3.3、worktree での実測)。
  2. **1 反復の手間**: §2.2 (計測後に記入)。
- **推奨する実装策** (§5、効果の見込みの大きい順): (1) 正則化の初期値・下限の変更 (`params.rs` の `RHO0`/`DELTA0`/`RHO_MIN`/
  `DELTA_MIN`、PIQP の finetune 相当の「残差が許容値内なら下限を 1e-13 へ」)、(2) Gondzio の多重中心性補正子を既定で有効化
  (既存の `ENOMOTO_T_IPM_GONDZIO`)、(3) 1 反復の手間 (§5 の 3 以降)。

## 1. 計測の方法

### 1.1 同じ問題を両方に解かせる

前処理後の問題 (内点法に渡す `A, b, c, l, u`: 固定列を除いた後、大きさの正規化 `β, γ` の前) をファイルに書き出す手段は無かった
ので、スクラッチの worktree (`<scratchpad>/wt`、HEAD から分岐) の `solve_box_lp_warm` に `ENOMOTO_DUMP_IPM_DIR` (バイナリ: `p, n, nnz`、
CSR、`b, c, l, u`) を足し、同じ worktree に `ENOMOTO_DEBUG_IPM` の出力へ求解回数・反復改良の回数・三角求解の時間を加えた
(`IPM solves=... refine_steps=... plain_solve_time=...`)。それ以外の経路は HEAD と同じ。双対化 (`ENOMOTO_T_XO_DUALIZE=1`、既定) で
内点法が双対 LP を解く問題では、書き出されるのも双対 LP である (Clarabel も同じものを解く)。

Clarabel には `min c^T x, A x = b (ZeroCone), -x_j + s = -l_j, x_j + s = u_j (NonnegativeCone)` として渡す
(`scripts/run_clarabel.py`)。許容誤差は enomoto の `EPS_ABS = EPS_REL = 1e-8` に合わせて `tol_feas = tol_gap_abs = tol_gap_rel = 1e-8`、
`max_iter = 200`。それ以外は Clarabel の既定 (Ruiz 平衡化 10 回、静的正則化 1e-8、動的正則化 ε = 1e-13 / δ = 2e-7、反復改良
最大 10 回 reltol 1e-13、presolve あり、`max_step_fraction = 0.99`)。線形ソルバは `qdldl` (1 スレッド) と `faer` (1 / 4 スレッド) を
記録した (Clarabel の `auto` は `flops / nnz(L) < 40` なら QDLDL、そうでなければ faer の超節点)。

- enomoto 側の時間は `ENOMOTO_DEBUG_IPM=1` の `IPM profile total` (記号分解 + 全反復。クロスオーバーを含まない)。
- Clarabel 側は `DefaultSolver(...)` の構築 (`t_setup`、KKT の組み立てと記号分解) と `solve()` (`t_solve`) の実時間。表の
  「時間」は `t_solve` (記号分解は Clarabel では `solve()` の中の `setup` に含まれる)。
- スクリプト: `<scratchpad>/scripts/{driver.py, run_enomoto.py, run_clarabel.py, ipmdump.py, summarize.py, variants_table.py,
  main_bench.sh}`。結果は `<scratchpad>/results/*.jsonl`。`<scratchpad>` =
  `/tmp/claude-0/-home-user-enomoto-solver/62f8f2eb-f61f-5117-b0a2-bc33ce60d30a/scratchpad`。

### 1.2 対象

- Netlib の小さな 60 問 (MPS 300 KB 未満): 反復数の比較と変種の実験 (1 スレッド、nice 19、ab14 と並行)。
- Netlib の中〜大 26 問 + Kennington 16 問 + Mittelmann の小さめ 6 問 (qap15, nug08-3rd, ns1688926, fome13, ns1687037, supportcase10):
  時間の比較 (4 コアを占有、ab14 の終了後、各 2 回の中央値)。

## 2. 比較

### 2.1 反復数 (Netlib の小さな 60 問、既定の設定)

enomoto の既定 (`RHO0 = DELTA0 = 0.1`) と Clarabel (qdldl) の反復数。時間は 1 スレッド・nice 19 で ab14 と並行して測ったので
参考値 (傾向だけ: 小さな問題では enomoto の 1 反復が 2〜10 倍遅い。rayon の並列ループの手間、§4)。

| 問題 | p × n | 反復 eno | 反復 Clarabel | 問題 | p × n | 反復 eno | 反復 Clarabel |
|---|---|---|---|---|---|---|---|
| adlittle | 53 × 134 | 57 | 31 | scfxm1 | 243 × 503 | 31 | 17 |
| ship04s | 213 × 1303 | 58 | 20 | scfxm2 | 495 × 1014 | 30 | 21 |
| ship04l | 313 × 1951 | 51 | 24 | scfxm3 | 742 × 1520 | 29 | 21 |
| forplan | 114 × 439 | 52 | 47 | e226 | 152 × 382 | 31 | 22 |
| agg2 | 282 × 508 | 34 | 23 | share1b | 91 × 227 | 27 | 21 |
| perold | 490 × 1264 | 200 (失敗) | 56 | bnl1 | 455 × 1334 | 36 | 86 |
| pilot4 | 322 × 917 | 200 (失敗) | 48 | fffff800 | 282 × 786 | 25 | 36 |
| stair | 245 × 421 | 25 | 26 | degen2 | 380 × 693 | 14 | 18 |

60 問の反復数の比 (eno / Clarabel) の幾何平均は **1.137** (失敗 2 問を除く 58 問)。enomoto が少ないのは bnl1、fffff800、scrs8、
sctap1 など 10 問程度。全表は `<scratchpad>/results/small_iters.jsonl` (`summarize.py`)。

### 2.2 時間 (中〜大の問題、4 コア)

(ab14 の終了後に計測して記入)

## 3. 反復数で負ける原因: 正則化 (近接項) の初期値と下限

### 3.1 症状

ship04s (既定): 7 反復目で相補積の広がりが `min(sz)/μ = 7e-3`、`max(sz)/μ = 98` になり、歩幅 `α = 0.05〜0.2` のまま μ が
`1e-4` 付近で 30 反復停滞する (Clarabel は 20 反復、歩幅 0.35〜0.9)。adlittle も同じ (8 反復目から歩幅 0.03〜0.5)。

```
IPM it=  7 alpha_p=1.83e-1 alpha_d=8.17e-2 mu=9.22e-5 sigma=4.37e-1 min(sz)/mu=7.0e-3 max(sz)/mu=9.8e1
IPM it= 19 alpha_p=3.04e-2 alpha_d=5.30e-2 mu=5.26e-5 sigma=8.24e-1 min(sz)/mu=7.5e-3 max(sz)/mu=3.4e2
IPM it= 28 alpha_p=4.26e-1 alpha_d=5.15e-1 mu=4.29e-5 sigma=2.43e-1 min(sz)/mu=2.7e-3 max(sz)/mu=1.1e2
```

このとき ρ, δ は `1e-2 〜 1e-5` で μ と同じ桁か大きい。IP-PMM の Newton 系は元の KKT 系ではなく近接項 `ρ(x − ξ)`、`δ(λ − y)`
を加えた系なので、ρ, δ が μ に比べて大きいうちは方向が元の問題の中心パスから外れ、中心性が壊れる。ρ, δ は相補性の相対減少率
`r` に比例してしか下がらない (`(1 − r)` 倍、停滞中は `(1 − 0.666 r)` 倍) ので、中心性を失って r が小さくなると ρ, δ も
下がらず、悪循環になる。

### 3.2 PIQP・Clarabel の既定との差

| | enomoto (`params.rs`) | PIQP (`settings.hpp`) | Clarabel |
|---|---|---|---|
| 初期 ρ (主の近接項) | `RHO0 = 1e-1` | `rho_init = 1e-6` | 静的正則化 `1e-8` (固定) |
| 初期 δ (双対の近接項) | `DELTA0 = 1e-1` | `delta_init = 1e-4` | 同上 |
| 下限 | `RHO_MIN = DELTA_MIN = 1e-10` | `reg_lower_limit = 1e-10`、残差が許容値内で更新が 7 回止まったら `reg_finetune_lower_limit = 1e-13` | ― |
| 停止のギャップ | `\|c·x + b·y + h·z\|` (相対 1e-8) | 既定では残差だけ (`check_duality_gap` は任意) | `\|pcost − dcost\|` (相対 1e-8) |
| 初期点 | `W = 1 + δ` の KKT を 1 回解く (PIQP 式) | 同じ | 等質埋め込みの標準初期化 |

enomoto の `RHO0 = DELTA0 = 0.1` は親モジュール (`interior_point.rs`) から引き継いだ値で、PIQP の既定より 3〜5 桁大きい。

### 3.3 終盤の床: 下限 1e-10 とギャップの停止判定

初期値を小さくすると (`ENOMOTO_T_IPM_REG0=1e-4`)、ship04s は 15 反復、adlittle 17 反復になるが、stair・bnl1 が 200 反復で
失敗する。trace を見ると 20〜27 反復目で `pres = 1e-9`、`dres = 1e-13` まで収束したあと、`gap = 3.6e-7` (stair)、`1.7e-7` (bnl1) が
一定のまま μ だけが `1e-300` まで潰れる。

```
stair (REG0=1e-4):
IPM it= 20 pres=1.09e-9 dres=3.08e-14 gap=1.95e-7 rho=1.0e-10 delta=1.0e-10
IPM it= 29 pres=1.09e-9 dres=5.33e-15 gap=3.61e-7 rho=1.0e-10 delta=1.0e-10   (以後 200 まで同じ)
```

主残差が `1.09e-9` で止まるのは、近接項のせいで Newton 方向が `A x − b = −δ(λ − y)` に収束するため (床 ≈ `δ‖λ − y‖`、
δ = 1e-10、‖λ − y‖ ≈ 10)。目的値で測るギャップ `c·x + b·y + h·z` には `r_p·y` の分 (1e-9 × ‖y‖ ≈ 300) が乗って 1e-7 となり、
`bnd_g ≈ 1e-8 × max(|c·x|, |b·y|, |h·z|)` を下回れない。既定の `0.1` から始めると、停滞中に `(1 − 0.666 r)` で下がる ρ, δ が
ちょうど収束の頃に 1e-10 に達するので、この床が見えにくかっただけである (perold・pilot4 の失敗の一因でもある)。

下限を PIQP の finetune と同じ `1e-13` にすると床は `1e-12` になり、stair 21、bnl1 30、tuff 24 反復で収束し、**perold (76)・
pilot4 (33) も収束する**。

### 3.4 変種の反復数 (Netlib の小さな問題、worktree の切り替え)

30 問 (上の表の問題を含む) で試した切り替え (いずれも HEAD に既にある環境変数、または worktree で足した
`ENOMOTO_T_IPM_REG0`/`RHO0`/`DELTA0`/`INIT_REG`)。反復数の比の幾何平均 (基準 = 既定 1.000)、「失敗」は 200 反復で収束しなかった数。

| 切り替え | 反復数の比 | 失敗 (既定は perold, pilot4 の 2) | 備考 |
|---|---|---|---|
| `REG0=1e-2` | 0.879 | 4 (bnl1, tuff, perold, pilot4) | |
| `REG0=1e-4` | 0.740 | 3 (stair 165, perold, pilot4) | ship04s 58 → 15、adlittle 57 → 17 |
| `RHO0=1e-6 DELTA0=1e-4` (PIQP の既定) | 0.780 | 3 (bnl1, perold, pilot4) | |
| `REG0=1e-6` | 0.865 | 3 (stair, tuff, perold) | 初期点の系まで小さい正則化で解くと stair で μ = 1.4e5 から発散 |
| `REG0=1e-8` | 0.851 (30 問) | 4 (stair, tuff, shell, perold, pilot4) | bnl1 36 → 173 |
| `REG0=1e-4` + 拡大系 (`ENOMOTO_IPM_AUGMENTED=1`) | 0.814 | 3 (bnl1, perold, pilot4) | stair 25 (正規方程式より精度が出る) |
| `REG0=1e-4` + 初期点の系だけ 0.1 (`INIT_REG=0.1`) | 0.851 | 4 | 初期点は原因でない |
| `REG0=1e-4` + Gondzio 2 回 | 0.648 | 3 (tuff 50, perold, pilot4) | |
| `REG_MODE=0` (PIQP の下げ方) | 1.111 | 2 | 第 8 回の比較どおり |
| `GONDZIO=2` だけ | 0.874 | 3 (forplan, perold, pilot4) | 第 1 回の比較の案 |
| `PROX_ALWAYS=1` だけ | 0.909 | 2 | |
| `IPM_AUGMENTED=1` だけ | 1.022 | 3 | |
| `REFINE=0` (反復改良なし) だけ | 1.000 | 2 | 反復数は変わらない (§4) |

60 問全部での候補 (`ENOMOTO_T_IPM_REG0=1e-4 ENOMOTO_T_IPM_RHO_MIN=1e-13 ENOMOTO_T_IPM_DELTA_MIN=1e-13`、以下「候補」):

| 設定 | 反復数の比 (基準 = 既定) | 反復数の比 (基準 = Clarabel) | 失敗 | 最終的な正否 (クロスオーバー後、基準値と相対 1e-6) |
|---|---|---|---|---|
| 既定 | 1.000 | 1.137 (58 問) | perold, pilot4 | 60/60 (2 問は最良の反復点から) |
| 候補 | 0.791 | 0.937 (60 問) | なし | 60/60 (pilot4 は二段解法へ戻る) |
| 候補 + `GONDZIO=2` | 0.690 | 0.818 (60 問) | なし | 60/60 |
| 候補 + `PROX_ALWAYS=1` | 0.787 | 0.932 (60 問) | なし | 60/60 |
| 下限 1e-13 だけ | 0.983 | ― | perold | pilot4 72 で収束 |

個別 (既定 → 候補 → 候補 + Gondzio): adlittle 57 → 17 → 14、ship04s 58 → 15 → 12、ship04l 51 → 15 → 14、forplan 52 → 20 → 18、
scfxm1 31 → 20 → 18、perold 失敗 → 76 → 63、pilot4 失敗 → 33 → 24、stair 25 → 21 → 16、bnl1 36 → 30 → 28。悪くなるのは
boeing2 14 → 18 (→ 12)、israel 20 → 23 (→ 19)、modszk1 20 → 23 (→ 18)、standmps 20 → 22 (→ 19) 程度。

## 4. 1 反復の手間で負ける原因

(ab14 の終了後に計測して記入)

## 5. 推奨する実装策 (効果の見込みの大きい順)

(計測後に確定)

## 6. 計測に使ったスクリプト

- `<scratchpad>/wt`: HEAD `338c0b4` から分岐した worktree。差分は `src/interior_point/boxed.rs` (問題の書き出し `ENOMOTO_DUMP_IPM_DIR`、
  調査用 `ENOMOTO_T_IPM_REG0`/`RHO0`/`DELTA0`/`INIT_REG`/`REFINE`、求解回数などの出力) と `src/interior_point/kkt.rs` (求解回数・
  反復改良回数・三角求解時間の累計)。`git -C <scratchpad>/wt diff` で見られる。
- `<scratchpad>/scripts/run_enomoto.py`: MPS を highspy で読み、`ipm_crossover` で解きながら書き出し、`ENOMOTO_DEBUG_IPM` の出力を解析。
- `<scratchpad>/scripts/run_clarabel.py`: 書き出した問題を Clarabel で解く (`--method qdldl|faer --threads N`)。
- `<scratchpad>/scripts/driver.py`: 問題集合を回して JSONL に追記。`summarize.py` (時間の表)、`variants_table.py` (反復数の表)。
- `<scratchpad>/scripts/main_bench.sh`: §2.2 の計測。
