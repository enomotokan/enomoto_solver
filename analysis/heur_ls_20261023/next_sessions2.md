# 次のセッションへの依頼文 その 2 (2026-10-23): HiGHS に解けて ENOMOTO に解けない問題

前の回 (`next_sessions.md`、結果は `next_sessions_results.md`) の続き。依頼文は「共通の前置き」と「依頼 I〜L」のどれか 1 つをつないだもの。

ba7f18b の計測 (4 スレッド・60 秒、既定 4 回) で、HiGHS が 60 秒以内に最適を証明し、ENOMOTO の既定が 1 回以上証明できなかった問題:

| 問題 | HiGHS | ENOMOTO 既定 (4 回) | mixoc (2 回) | 扱い |
|---|---|---|---|---|
| mzzv42z | 50.1 秒 | 0/4 | 0/2 | 前の回 F (GR_RESTART で解は改善) |
| physiciansched6-2 | 10.8 秒 | 0/4 (解なし) | 0/2 | 前の回 D は解だけ。下界は依頼 K |
| roll3000 | 46.3 秒 | 0/4 | 0/2 | 前の回 G は手当てなし。依頼 L |
| timtab1 | 59.9 秒 | 0/4 | 0/2 | 前の回 B (別の機械では HiGHS も 80 秒)。扱わない |
| 10teams | 1.6 秒 | 3/4 (24.3・22.3・44.0 秒、1 回 954 で時間切れ) | 2/2 (42.8・15.0) | 依頼 I |
| hypothyroid-k1 | 12.8 秒 | 3/4 (32〜37 秒、1 回時間切れ) | 2/2 (37.7・34.2) | 依頼 J |
| h80x6320d | 17.4 秒 | 3/4 (53〜60 秒) | 1/2 | 前の回 H (再スタート回数)。依頼 L |
| beasleyC3 | 25.8 秒 | 3/4 | 1/2 | 前の回 H (CUT_STRONG_WIN5) |
| neos-960392 | 13.9 秒 | 3/4 | 0/2 | 前の回 E (LP_PROBE_FIRST_FULL) |
| mc11 | 55.1 秒 | 3/4 | 2/2 | 扱わない (HiGHS と同程度) |

| 依頼 | `<話題>` |
|---|---|
| I | 10teams |
| J | hypothyroid |
| K | physbound |
| L | treeproof |

---

## 共通の前置き (どの依頼にも付ける)

あなたは enomoto_solver (Rust 製の MIP ソルバー) の調査・実装セッションです。
報告・メモ・コミットメッセージは日本語で書いてください。メモは事実と推測を分けて書いてください。

### 背景 (読むこと)

- 土台はブランチ `inv/heur-ls-20261023` の最新 (この依頼文を足したコミット以降)。
- 前の回 (依頼 A〜H) の結果は `analysis/heur_ls_20261023/next_sessions_results.md`。各切り替えは各ブランチにある (土台にはない)。使うなら自分のブランチに cherry-pick してよい。
- 経緯と計測の結果は `analysis/heur_ls_20261023.md` (特に追記 4〜8)。
- 試作の切り替え:
  - `ENOMOTO_MIP_FJ_LMIP_MIX=1`: FJ のスレッドで FJ と Local-MIP + 連鎖の手に時間を分け合う (`src/mip/heuristics.rs` の `fj_lmip_mix`、`src/mip/localmip.rs`)。
  - `ENOMOTO_MIP_SYM_ORIG_CUT=1`: 対称性の行への並べ替えに失敗した解を、値だけ打ち切り値に使う (`src/mip/solver.rs` の `orig_best`)。
- この 2 つを合わせた形を **mixoc** と呼ぶ。既定にする候補。
- 計測の表 (4 スレッド・60 秒、40 問 + 16 問、2 組):
  - `analysis/heur_ls_20261023/table_bench_mix6.md` (ba7f18b: newdef6・mix6・mixoc6)
  - `table_bench_mix5.md`・`table_bench_mix4.md`
- 各回の生の結果はブランチ `inv/heur-ls-bench-20261023`・`-s2`〜`-s8` の `benchmarks/*_ba7f18b_*` と `*_9e34b8e_*` (JSON)。
- HiGHS の値 (状態・目的値・時間) も同じ JSON の `highs` に入っている (HiGHS 1.15.1、4 スレッド・60 秒、mip_rel_gap 0)。

### 作業の決まり

- 作業はブランチ `inv/<話題>-20261023` (inv/heur-ls-20261023 の最新から作る) だけで行い、ここにだけ push する。develop や inv/heur-ls-20261023 には push しない。
- ビルドとインストール:
  - `pip install highspy==1.15.1 maturin scipy`
  - `maturin build --release -o dist`
  - `pip install --no-deps --force-reinstall dist/*.whl`
- テスト: `L=$(python3 -c "import sysconfig;print(sysconfig.get_config_var('LIBDIR'))"); V=$(python3 -c "import sysconfig;print(sysconfig.get_config_var('LDVERSION'))"); RUSTFLAGS="-L $L -lpython$V" LD_LIBRARY_PATH=$L cargo test --release --lib`。これを通すこと。
- MPS (コミットしない): `git fetch origin inv/mps-cache-20261021 && mkdir -p .miplib_cache && for f in $(git ls-tree --name-only origin/inv/mps-cache-20261021 mps/); do git show origin/inv/mps-cache-20261021:$f | gunzip > .miplib_cache/$(basename $f .gz); done`
- 1 問の実行: `analysis/heur_ls_20261023/run_cfg.sh <問題> <秒> <名前> [ENV=VAL ...]`
  - ログは `analysis/heur_ls_20261023/logs/<問題>_<名前>_<n>.log`。`ENOMOTO_MIP_LOG=1` は付く。
  - 最後の行の JSON に、状態・目的値・時間・最初の解。
- 診断の切り替えの例:
  - `ENOMOTO_MIP_FJ_DEBUG=1`
  - `ENOMOTO_MIP_LOCALMIP_DEBUG=1`
  - `ENOMOTO_MIP_TREE_DIAG=1`
  - `ENOMOTO_MIP_NO_SYMMETRY=1`
  - ほかは `src/` を `env_str!` で探す。
- 多くの問題で測るときは `analysis/heur_ls_20261023/bench_mix.sh <名前> <回> [ENV=VAL ...]` を使う。
  - `ENOMOTO_BENCH_ONLY="問題 ..."` で問題を絞る。`ENOMOTO_BENCH_SHA=<ビルドしたコミット>` を付ける。
  - 結果は `benchmarks/`、集計は `analysis/heur_ls_20261023/agg_bench.py`。
- 長い計測は Bash の run_in_background (timeout 7200000) で動かす。
  - sleep を連ねない。setsid・nohup で切り離さない。
  - ソルバーは同時に 1 つ。計測中にビルドしない。
- 比べ方:
  - 4 スレッド・60 秒。
  - 同じ機械で、既定 (何も付けない) と mixoc (`ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1`) を、少なくとも 2 回ずつ。
  - 1 回ごとの揺れが大きい問題が多いので、主張は「2 回とも同じ向き」のものだけにする。
- 新しい手当ては、既定を変えない切り替え (環境変数) で入れる。既定にする判断は親が 40 問 + 16 問の計測で行う。
  - 自分で 40 問 + 16 問を回す必要はない。回すなら bench_mix.sh で 1 組だけ。
- 目的値の不一致 (HiGHS の最適値と違う「最適」) が出たら、最優先で報告する。

### 終わったら

- メモ `analysis/<話題>_20261023.md` を書く。
- メモ・台本・ログをブランチに push する。
- 親セッションがあれば send_message (to: @parent) で要点を 5 行以内で送り、同じ内容を最後の返答にする。要点は次の 4 つ。
  - 原因
  - 手当て (切り替えの名前)
  - 効果 (2 回ずつの数)
  - 既定にする候補か

---

---

## 依頼 I: 10teams の HiGHS との時間の差 (`<話題>` = 10teams)

### 事実

- HiGHS は 1.6 秒で最適 924 を証明する。
- ENOMOTO の既定は 22〜44 秒で証明、1 回は 954 のまま時間切れ。mixoc は 15.0・42.8 秒。
- 前に mix3 で 10teams が良くなった記録がある (`analysis/heur_ls_20261023.md` 追記の mix3 の節)。

### やること

1. HiGHS のログと比べ、HiGHS が 1.6 秒で何をしているか (前処理後の大きさ、根の下界、最初の解の出どころと時刻、ノード数) を示す。
2. ENOMOTO が 924 を見つける時刻と出どころ、下界が 924 に届く時刻を、既定と mixoc で 3 回ずつ示す。遅いのは解か下界かを分ける。
3. 一番効く手当てを 1 つ切り替えで入れ、既定と mixoc で 3 回ずつ時間の分布を比べる。

---

## 依頼 J: hypothyroid-k1 の HiGHS との時間の差 (`<話題>` = hypothyroid)

### 事実

- HiGHS は 12.8 秒で最適 −2851 を証明する。
- ENOMOTO の既定は 32〜37 秒で証明、1 回は −2851 に届いたまま時間切れ。mixoc は 34.2・37.7 秒。
- 前の調査: `analysis/hypothyroid_sb_20261020.md`、`analysis/regress_nursesched_hypothyroid_ns1116954_neos950242_20261020.md`。先に読む。

### やること

1. HiGHS のログと比べ、時間の差がどこにあるか (前処理・根・最初の解・木) を時刻つきで示す。
2. −2851 を見つける時刻と、証明 (下界が届く) までの時間を分ける。既定と mixoc で 3 回ずつ。
3. 一番効く手当てを 1 つ切り替えで入れ、既定と mixoc で 3 回ずつ時間の分布を比べる。

---

## 依頼 K: physiciansched6-2 の根の下界 (`<話題>` = physbound)

### 事実

- HiGHS は 10.8 秒で最適 49324。HiGHS は前処理とカットで根の下界を 49324 まで上げる。
- ENOMOTO の根の下界は 48587 のまま。
- 前の回 D の調べ: 根のカットのラウンド 1 の LP が 1.2 秒で時間切れになり、カットが 1 ラウンドで止まる。解の手当て (`ENOMOTO_MIP_FJ_NOSOL_EXTEND`) はブランチ `inv/physiciansched-20261023` にある。メモ `analysis/physiciansched_20261023.md` と `analysis/physiciansched6-2_20261020.md` を先に読む。

### やること

1. ラウンド 1 の LP が時間切れになる理由 (期限の決め方、LP の大きさ、反復数、カットの本数) を示す。
2. 期限を緩めたとき、カットが何ラウンド回り、根の下界がどこまで上がるかを示す。HiGHS のカットの種類と比べる。
3. 下界を上げる手当てを 1 つ切り替えで入れる。`FJ_NOSOL_EXTEND` を cherry-pick して組み合わせてよい。既定と mixoc で 2 回ずつ、根の下界・最終の値・証明を比べる。
4. ほかの問題で根の時間が延びすぎないか、根のカットの多い問題 (例: neos-911970・mzzv42z・roll3000) で 1 回ずつ確かめる。

---

## 依頼 L: 木の探索での証明の遅さ (roll3000・h80x6320d) (`<話題>` = treeproof)

### 事実

- roll3000: HiGHS 46.3 秒で証明。ENOMOTO は最適値 12890 に届くが 4/4 時間切れ。前の回 G の調べ: 差は根でなく木の下界の伸び。60 秒の下界 HiGHS 12844、ENOMOTO 12551〜12819。共有の待ち行列から取ったノードは最初の LP が 2000〜5600 反復 (`ENOMOTO_MIP_TREE_POOL_BASIS` で 100〜200 になるが下界は揺れの範囲)。メモ `analysis/roll3000_20261023.md` (ブランチ `inv/roll3000-20261023`)。
- h80x6320d: HiGHS 17.4 秒、ENOMOTO 53〜60 秒。前の回 H の調べ: 再スタートの回数で決まる。メモ `analysis/proofvar_20261023.md` (ブランチ `inv/proofvar-20261023`)。

### やること

1. 2 問で、木の 1 秒あたりのノード数・ノードあたりの LP 反復・下界の上がり方を HiGHS と比べる (HiGHS のログのノード数と時刻)。
2. 下界を上げているのはどのノードの処理か (最良優先の選び方、潜りとの比率、スレッド間の分け方) を調べ、HiGHS との差の一番大きいところを特定する。
3. h80x6320d の再スタートの回数を決めているものを調べる。
4. 手当てを 1 つ切り替えで入れ、既定と mixoc で 3 回ずつ、60 秒での証明の回数・時間・下界を比べる。

