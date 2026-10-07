# ex10 が「解けない」件の調査: 傾き・切片双対二段解法と既定 (`auto`) の同時実行 (2026-10-06)

対象: Mittelmann LPopt の `ex10.mps` (69,608 行 × 17,680 列, 1.16M 非零。前処理後 63,012 × 15,932)。
ブランチ `claude/confident-faraday-0cj5gq` (HEAD `6aedacc`)、**src は変更していない**。計測は 4 コア (Intel Xeon 2.80 GHz) /
15 GB の計算機、`.venv` (maturin develop --release、highspy 1.15.1)。過去の記録は benchmarks/results_120_20260928.md
(ex10 83.8 s、既定 = 二段解法のみ) と benchmarks/crossover/mitt_small/results.jsonl (2026-10-06 01:31、slope_intercept 81.0 s、
race 79.1 s、ipm_crossover 300 s 時間切れ)。

## 0. 要約

- **二段解法 (`root_solver="simplex"`) 自体は退行していない。** HEAD で ex10 は 191.7 s で最適 (目的 99.99999999999986、
  基準 100.00000000000013 と相対 2.7e-15)。過去記録の 81 s との差はこの計算機の遅さ (前処理 1.50 s 対 0.68 s = 2.2 倍。
  HiGHS の双対単体法もこの計算機で 156.6 s 対 過去 90 s) で説明でき、1 反復あたりの手間・反復数は §2 のとおり過去の調査と同じ。
  既定が二段解法だけだった最後の記録 (`a0b0100` 直前の `mitt_small`) と同じコミットを別 clone でビルドして走らせた結果も §1.3。
- **ex10 が「解けない」のは既定の `root_solver="auto"` (ef206af、2026-10-05 で既定になった同時実行) の内点法側が原因。**
  前処理後の行数 63,012 ≥ `RACE_MIN_ROWS` = 5,000 なので、二段解法と内点法 + クロスオーバーを同時に走らせる。内点法は
  63,012 × 63,012 の正規方程式 `A D A^T` を疎 Cholesky で分解しようとし、三つ組 36.5M (上限 `NORMAL_MAX_TRIPLETS` = 40M を
  **わずかに下回る**ので正規方程式が選ばれる)、パターン 16.7M、**AMD 後の `nnz(L)` = 753M** (`L` だけで 6.0 GB) になる。
  数値分解は 1 回も終わらず (設定 63〜67 s、その後 130 s 以上かけても 1 反復目に達しない)、**プロセスの常駐メモリは 9.9 GB**
  に達する (二段解法だけなら 0.31 GB)。
  - この計算機 (15 GB) では二段解法側が 193.7 s で勝って最適解を返す (`race_simplex_won`)。
  - README の計測環境 (WSL2、メモリ上限 12 GB) や、`scripts/crossover_bench/run.py` の `RLIMIT_AS` = 13.5 GB 下では、
    内点法側の 10 GB 近い確保が二段解法のスレッドと同じプロセスなので、**OOM で落ちる・スワップで二段解法まで遅くなる・
    `RLIMIT_AS` で確保に失敗して abort する**のいずれかになり、「解けない」になる。この計算機でも仮想サイズは 11.1 GB で、
    13.5 GB の制限にはぎりぎり収まる程度。
  - 内点法 + クロスオーバー単独 (`ipm_crossover`) は 600 s で時間切れ (過去の記録どおり)。
- **HiGHS との差**: HiGHS の双対単体法は 156.6 s / 25,974 反復 (この計算機、他の計測と同時実行)。HiGHS の IPM (IPX) は
  **「Dualized model: yes」で双対化してから**正規方程式を組むので、系の次元が 15,895 になり、16 反復 25.5 s で収束し、
  クロスオーバーも含めて §1.4 の時間で解ける。enomoto の内点法は双対化せず行数側の 63k 次元で組むのが根本の違い。
- **推奨する実装策** (§3): (1) `NormalKkt::new`/`IpmKkt::new` に記号分解後の `nnz(L)` の上限 (メモリ見積もり) を入れ、
  超えたら内点法を諦める (同時実行では二段解法だけにする)。(2) `p ≫ n` の問題は内点法でも双対化する (HiGHS IPX と同じ)。
  (3) 同時実行の内点法側に時間・メモリの予算を持たせる。

## 1. 計測

計測は `scripts/crossover_bench/run.py --worker` と同じ流れ (highspy で MPS を読み、`_build_our_model` で `PyModel` を組み、
`solve` の前後の実時間。前処理・後処理を含む) を単独スクリプトで行い、`_core.last_solve_events()` の節目を記録した。

### 1.1 HEAD の 3 方式 (600 s 制限、各 1 回)

| 方式 | 時間 | 状態 | 目的 | 節目 (秒) | 常駐メモリの最大 |
|---|---|---|---|---|---|
| `simplex` (二段解法のみ) | **191.7 s** | optimal | 99.99999999999986 | presolve_end 1.50、stage_a_skipped 1.51、simplex_end 191.66 | 0.31 GB |
| `auto` (既定、同時実行) | **193.7 s** | optimal | 100.00000000000018 | presolve_end 1.60、race_simplex_won 193.72 | **9.86 GB** (仮想 11.1 GB) |
| `ipm_crossover` | 時間切れ (600 s) | ― | ― | §1.2 | §1.2 |

`auto` の内点法側のログ (`ENOMOTO_DEBUG_IPM=1 ENOMOTO_DEBUG_RACE=1`):

```
NormalKkt: dense_cols=0 (threshold 139) triplets=36473360 pattern nnz=16655617 built in 28.70s
NormalKkt: symbolic (AMD) nnz(L)=753274655 at 37.94s
IPM kkt=normal nnz(A)=1095016 nnz(L)=753274655 setup=62.81s
RACE Simplex finished at 192.114s with Some(Optimal)
```

`IPM t=... it=0` の行は出ない = 初回の数値分解が 192 s 時点でも終わっていない。稠密列は 0 本 (列の非零は平均 65〜73 で
均一、閾値 139)。`A A^T` のパターン密度は 0.8% だが、AMD 順序の fill-in で `L` は 63k × 63k の下三角の 38% が埋まる。

### 1.2 内点法 + クロスオーバー単独 (`ipm_crossover`、600 s)

`ENOMOTO_DEBUG_IPM=1 ENOMOTO_DEBUG_CROSSOVER=1`、他の計測 (HiGHS、二段解法) と同時実行:

```
NormalKkt: dense_cols=0 (threshold 139) triplets=36473360 pattern nnz=16655617 built in 28.16s
NormalKkt: symbolic (AMD) nnz(L)=753274655 at 37.29s
IPM kkt=normal nnz(A)=1095016 nnz(L)=753274655 setup=64.31s
IPM t=468.14s it=  0 pobj=1.3820730506e1 pres=1.44e-1 dres=6.61e-1 gap=1.83e3 rho=1.0e-1 delta=1.0e-1
```

- 設定 (CSC 化・三つ組 36.5M の行き先の二分探索 28 s、AMD + 記号分解 9 s、`L` 6.0 GB と作業領域の確保) に 64 s。
- **初期化の数値分解 1 回 (+ 初期点の求解) に約 400 s** (`it=0` の行が 468 s)。反復 1 回につき分解 1 回なので、収束に
  HiGHS 並みの 16〜30 反復かかるとしても **2〜3 時間**、`ENOMOTO_T_IPM_MAX_ITERS` = 200 なら 22 時間。600 s では 1 反復目の
  分解の途中で時間切れ (過去の記録と同じ)。
- 常駐メモリの最大 9.7 GB、仮想 11.1 GB (`L` 6.0 GB + 数値分解の作業領域 + パターン・三つ組の行き先 0.3 GB + …)。
- 疎 Cholesky の数値分解は既定で逐次 (`ENOMOTO_T_FACTOR_SEQ=1`、docs/crossover.md 第 2 回) なので、同時実行でも
  CPU は 1 コアしか使わないが、メモリは全部このスレッドが使う。

拡大系 (`ENOMOTO_IPM_AUGMENTED=1`、`[D A^T; A -δI]`、次元 n + p) の場合:

```
IPM kkt=augmented nnz(A)=1095016 nnz(L)=102194209 setup=2.99s
IPM t=29.67s it=  0 pobj=1.3820730506e1 pres=1.44e-1 dres=6.61e-1 gap=1.83e3 rho=1.0e-1 delta=1.0e-1
IPM t=55.24s it=  1 pobj=4.0794712441e1 pres=8.10e-1 dres=1.90e-2 gap=5.97e2 rho=1.3e-2 delta=4.2e-2
```

**`nnz(L)` = 102M (正規方程式の 1/7.4、0.8 GB)、設定 3 s、1 反復 (分解 1 回) 約 25.6 s、常駐 1.7 GB。** 正規方程式の 1 反復
約 400 s の 1/15 だが、HiGHS 並みの 16〜30 反復でも 7〜13 分で、600 s には収まらない (2 反復で打ち切った)。
AMD が列 (16k) を先に消去すれば `A^T D A` の 16k 次元の密ブロック (127M) になるので、この 102M はほぼその下限 =
**双対化した正規方程式と同じ大きさ**。ex10 で内点法を使うには、さらに (a) 数値分解の並列化 (いまは逐次)、(b) 双対化して
正規方程式 16k 次元 (密 Cholesky 16k³/3 ≈ 1.4e12 flop、BLAS で数秒) にする、が要る。

### 1.3 退行の確認: `a0b0100` (既定が二段解法だけだった最後の ex10 記録と同じコミット) のビルド

`a0b0100` (benchmarks/crossover/mitt_small/results.jsonl の ex10 81.0 s を出したビルドの元) を `git worktree` の別 clone に
ビルド (`venv2`) し、同じ手順で `simplex` を走らせた: **206.8 s、最適、目的 99.99999999999986** (presolve_end 1.69 s。他の 2 本の
計測と同時実行)。HEAD の 191.7 s / 198.7 s と同じ範囲で、a0b0100 → HEAD の差分 (`src/simplex/slope_intercept_dual.rs` に
クロスオーバーの仕上げ用の `solve_slope_intercept_dual_from_basis`/`warm_start_basis` を足しただけ。`WARM_BASIS` が `None` の
通常経路は不変) に退行はない。同じ記録の 81 s とは、この計算機の 1 コア性能の差 (前処理 0.68 → 1.5〜1.7 s、HiGHS 90 → 157 s)。
ゆえに「解けない」は二段解法の変更によるものではなく、既定が `auto` になった ef206af (2026-10-05) 以降の同時実行の副作用 (§2)。

### 1.4 HiGHS 1.15.1 (同じ計算機)

| 設定 | 時間 | 反復 | 備考 |
|---|---|---|---|
| 既定 (双対単体法、threads=1) | 156.6 s | 25,974 | presolve 2.8 s、前処理後 62,931 × 15,895。enomoto の内点法単独と同時実行 |
| `solver=ipm, run_crossover=on` (threads=2) | **75.6 s** | IPM 16、クロスオーバー 12,552 | IPX 74.7 s = IPM 25.5 s + クロスオーバー約 47 s (双対の押し出し 14,618、主の押し出し 1,178)。双対化あり。enomoto の内点法単独と同時実行 |

HiGHS IPX のログ抜粋:

```
IPX model has 62931 rows, 15895 columns and 1031920 nonzeros
    Dualized model:                                     yes
  16*  -9.79999992e+01  -9.80000001e+01   3.93e-15   1.03e-09  9.06e-09      25.5
Running crossover as requested
    Number of dual pushes required:                     14618
```

IPX は行数 ≫ 列数の問題を双対化して (次元 15,895 の正規方程式) 16 反復・25.5 s で収束する。

### 1.5 二段解法の中身 (HEAD、`ENOMOTO_DEBUG_EXT_ITERS=1 ENOMOTO_PROF_PHASES_EXT=1`)

計測付きの走行 (他の 2 本の計測と同時実行、198.7 s / 最適): **主ループ 22,259 反復、`bland_mode` に入らず**、段階 A は省略
(S が空)、仕上げ (polish) 0 反復、再分解 72 回 (全て周期 = clock、ピボット破棄・fill 超過・ドリフトによる再分解は 0)、
費用シフト 0、退化ピボット 91.9% (Harris 窓 1.26 候補)。1 反復 8.76 ms (99.4% を計時済み):

| 段 | 時間 | 割合 | 1 反復あたり |
|---|---|---|---|
| FTRAN (入る列) | 62.8 s | 32.2% | 2.82 ms |
| x_B 更新 | 34.9 s | 17.9% | 1.57 ms |
| chuzr | 25.8 s | 13.2% | 1.16 ms |
| PRICE | 18.7 s | 9.6% | 0.84 ms |
| BTRAN (rho_p) | 18.0 s | 9.3% | 0.81 ms |
| 再分解 | 12.3 s | 6.3% | 0.55 ms |
| chuzc1 / DSE 更新 / FT 更新 / d 更新 / BFRT | 5.8 / 5.7 / 5.7 / 3.3 / 0.8 s | 3.0 / 2.9 / 2.9 / 1.7 / 0.4% | |

FTRAN の結果密度は 0.91 (ほぼ密)、BFRT 側 0.44。反復数 22,259 は HiGHS の 25,974 より少なく (0.86 倍)、内訳は
analysis/square41_ex10_20260925_122411.md の ex10 (23,651 反復、6.09 ms/反復: FTRAN 32%、x_B 更新 18%、BTRAN 13%、chuzr 11%)
と同じ形。つまり二段解法の経路・反復数は変わっておらず、1 反復の絶対時間 (8.76 ms 対 6.09 ms) だけが計算機の差で延びている
(HiGHS も 3.41 → 6.03 ms/反復)。プラトー検出・Bland 規則・特異化・再分解の過多は起きていない。

## 2. 根本原因

1. **既定 `auto` の同時実行が、ex10 では内点法側のメモリ (約 10 GB) と CPU を無駄に使う。** 同時実行は「先に結論を出した側を採る」
   設計で、負けた側の手間は時間には響かない前提だが、内点法側が **数値分解 1 回に数分・6 GB の `L`** を要する問題では、
   (a) メモリ上限のある環境ではプロセス全体 (二段解法を含む) が落ちる、(b) 上限がなくてもスワップ・メモリ帯域で二段解法が遅くなる。
   ex10 は正規方程式の三つ組が 36.5M で `NORMAL_MAX_TRIPLETS` = 40M をすり抜け、記号分解後の `nnz(L)` には上限がない
   (`src/interior_point/kkt.rs:468, 536, 577-580`)。拡大系 (`AugKkt`) に落ちた場合も `nnz(L)` の上限はない。
2. **内点法が双対化しない。** `p = 63,012 ≫ n = 15,932` なので、正規方程式 `A D A^T` (63k 次元) ではなく双対の `A^T D A` (16k 次元、
   密でも 127M 非零 ≈ 1 GB) の方が桁違いに小さい。HiGHS IPX はこれを自動で選ぶ (「Dualized model: yes」)。
   enomoto は単体法側に `dualize` (`src/simplex/dualize.rs`、行数/列数の比で双対化) を持つが、内点法 + クロスオーバーには適用されない
   (`solve_ipm_crossover` は `std` をそのまま `reduce_fixed` して渡す)。
3. 二段解法そのものには問題がない (§1.1、§1.3、§1.5)。

## 3. 推奨する実装策 (効果の大きい順)

| # | 策 | 変更箇所 | 既定値の候補 | 効果 | 退行リスク |
|---|---|---|---|---|---|
| 1 | **`nnz(L)` と作業領域の上限で内点法を諦める。** `NormalKkt::new` で記号分解の直後に `chol_symbolic.len_values()` を見て、`8·nnz(L) + numeric_buf` がメモリ予算を超えたら `None` を返す。`AugKkt::new` も同様に `Option` を返すようにし、`IpmKkt::new` が `None` のとき `solve_box_lp*` は `Status::NotSolved` で即戻る (クロスオーバーは `None` → `ipm_crossover` 単独なら二段解法で解き直し、同時実行なら二段解法だけが走る)。予算は `ENOMOTO_T_IPM_MAX_FACTOR_NNZ` (既定 1e8 = 0.8 GB、または利用可能メモリの 1/4) で調整可能に | `src/interior_point/kkt.rs` `NormalKkt::new` (`:577-580` の直後)、`AugKkt::new` (`:300-306`)、`IpmKkt::new` (`:777-785`)、`src/interior_point/boxed.rs` `solve_box_lp_scaled` (`:341` の `IpmKkt::new` の直後に早期 return)、`src/simplex/crossover.rs` `solve_ipm_crossover_with` (`:585` 付近で `NotSolved` → `None`) | 1e8 (ex10 の 7.5e8 は弾く。pds-20 の 3.4M、Netlib・Kennington は全て 1e6 台以下なので不変) | ex10 の `auto` のメモリ 9.9 GB → 0.3 GB、`IpmKkt::new` の 63 s の CPU も省ける。12 GB 環境で「解けない」が「191 s で最適」になる | 上限を超える問題では内点法側が走らなくなるだけ (二段解法の結果は不変)。Netlib/Kennington の 109 問は `nnz(L)` が小さく不変 |
| 2 | **内点法の双対化。** `p > κ·n` (κ = 2〜4) の標準形は双対問題 (箱型制約付きの `max b^T y` ...) を内点法で解き、`x` と `y` を入れ替えて渡す。単体法側の `dualize` (`src/simplex/dualize.rs`) の変換を再利用し、クロスオーバーは元の問題の `(x, y, s)` で行う | `src/simplex/crossover.rs` `solve_ipm_crossover_with` の先頭 (`:513` `reduce_fixed` の前) で `dualize` を適用、`solve_box_lp` の結果を元の変数に戻す | κ = 3 (ex10 は 3.96、cont1・irish も該当。`ENOMOTO_DUALIZE` 相当の試験用切り替えを付ける) | ex10 の正規方程式が 16k 次元になり、HiGHS 並み (16 反復・25 s) に収束する見込み。クロスオーバーが続くので全体で解けるかは別途 (HiGHS のクロスオーバーは §1.4) | 双対化後の箱型制約・自由変数の扱いを内点法で正しく組む必要がある。二段解法の経路は不変 |
| 3 | **同時実行の内点法側に予算を持たせる。** 内点法側は `IpmKkt::new` の後に `factor_nnz()` を見て、`nnz(L) > c · nnz(A)` (c = 50〜100) なら同時実行では内点法を走らせない。策 1 を入れれば自動的に満たされるが、策 1 の予算が緩いときの保険 | `src/simplex/race.rs` `solve_race` (内点法スレッドの起動前に見積もる)、または `crossover.rs` | c = 100 | 同時実行のメモリの頭打ち | なし (負けるはずの側を止めるだけ) |
| 4 | **正規方程式と拡大系を記号分解の `nnz(L)` で選ぶ。** `IpmKkt::new` で両方の記号分解 (AMD のみ、数値分解なし。ex10 で正規 9 s・拡大 3 s) を取り、`nnz(L)` の小さい方を使う (いまは三つ組 40M の閾値だけ。三つ組の数は列の非零の 2 乗和で fill-in の指標としては弱い) | `src/interior_point/kkt.rs` `IpmKkt::new` (`:777-785`)、`NormalKkt::new` の記号分解 (`:577`) を数値用バッファの確保 (`:580-582`) から分離 | 比 2 倍以上小さければ拡大系 | ex10 は拡大系 (`nnz(L)` 102M、1 反復 25.6 s、常駐 1.7 GB) に落ち、メモリ 9.8 GB → 1.7 GB、1 反復 400 s → 26 s。ただし 600 s では収束しない (策 2 が要る) | 拡大系の LDLᵀ は正則化が小さいと不安定 (docs/crossover.md: greenbea、pilot4)。小さな問題では今の選択を変えないよう `nnz(L)` が大きいときだけ切り替える |
| 5 | **数値分解の並列化を大きい `nnz(L)` で戻す。** `factor_par()` は既定で逐次 (`ENOMOTO_T_FACTOR_SEQ=1`、小さな問題では並列の方が遅かった) だが、`nnz(L)` が 1e7 を超える超節点分解では faer の並列が効く | `src/interior_point/kkt.rs:33-39` | `nnz(L) > 1e7` で `KKT_PARALLELISM` | 同時実行では内点法側に 3 コアがあるので、ex10 の拡大系 26 s/反復 → 10 s 台の見込み (未計測) | Netlib・Kennington は閾値未満で不変 |

策 1 が本命で、変更は小さい (`Option` を返す・早期 return)。ex10 を「解けない」から「二段解法で 190 s (この計算機) / 80 s (README の計算機)」に
戻すには策 1 (または 3) だけで足りる。策 2・4・5 は内点法を ex10 の大きさで実用にするためのもので、HiGHS IPX (双対化、16 反復 25 s、
クロスオーバー込み 76 s) に並ぶには策 2 が要る。

## 4. 環境構築の手順 (この調査で使ったもの)

```sh
cd /home/user/enomoto_solver
python3 -m venv .venv && .venv/bin/pip install maturin highspy numpy
.venv/bin/maturin develop --release            # python/enomoto_solver/_core*.so
mkdir -p .mittelmann_cache/raw && cd .mittelmann_cache/raw
curl -sS -L -o ex10.mps.bz2 https://plato.asu.edu/ftp/lptestset/ex10.mps.bz2 && bzip2 -dk ex10.mps.bz2
```

- `ex10.mps`: `/home/user/enomoto_solver/.mittelmann_cache/raw/ex10.mps` (35 MB。`scripts/run_mittelmann_benchmark.py` の
  `.mittelmann_cache/raw/ex10.mps.bz2` と同じ置き場所なので `scripts/crossover_bench/run.py --set mittelmann --only ex10` もそのまま使える)。
- 単発計測: `ENOMOTO_DEBUG_RACE=1 ENOMOTO_DEBUG_IPM=1 .venv/bin/python -I <worker.py> ex10.mps {simplex|default|ipm_crossover}`
  (worker は `scripts/crossover_bench/run.py` の `worker()` と同じ手順。この調査のスクリプトはスクラッチ領域にあり、リポジトリには入れていない)。
- HiGHS: `highspy` 1.15.1、`solver=ipm`, `run_crossover=on`, `log_file` で IPX のログを取る。
- 別コミットの比較は `git worktree add <scratch>/wt <commit>` + 別 venv (`venv2`) に `maturin develop --release`。
