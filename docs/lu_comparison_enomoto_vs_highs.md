# LU 分解アルゴリズム比較: ENOMOTO-Solver vs HiGHS

ENOMOTO-Solver (`src/simplex/lu.rs`) と HiGHS 本家 (`util/HFactor.cpp`,
`util/HFactorRefactor.cpp`) のLU分解・更新・求解実装を読み比べた結果と、
ENOMOTO 側で高速化に参考になりそうな点をまとめる。

対象コード:

- ENOMOTO: `src/simplex/lu.rs` (~2660行), `src/simplex.rs` (呼び出し側)
- HiGHS: `highs/util/HFactor.cpp`, `HFactorRefactor.cpp`,
  `HFactorExtend.cpp`, `HFactor.h`, `HFactorConst.h`,
  `highs/util/HVectorBase.h`

---

## 1. 全体アーキテクチャの違い

| 項目 | ENOMOTO | HiGHS |
|---|---|---|
| 疎行列構造 | `Vec<BTreeMap<usize, f64>>` (行) + `Vec<BTreeSet<usize>>` (列ミラー) | CSR形式 `mc_*` (列) + `mr_*` (行)、**両方を1つのフラット配列で管理** |
| 次数管理 | バケット配列 `Vec<VecDeque>` + `col_bucket_pos` | **リンクリスト** `col_link_first/next/last`, `row_link_first/next/last` |
| ピボット選択 | Markowitz score `(r-1)(c-1)` 最小 + 安定性床 + **search limit = 256** (§2.5) | 同じ Markowitz、ただし **search limit = 8** の打ち切り |
| 前処理 | なし (バケットの早期終了のみ) | **`buildSimple()` で単位列・行列シングルトン・列シングルトンを O(nnz) で剥離** |
| 密行列dispatch | `is_dense_input()` → `faer::PartialPivLu` | なし (常にMarkowitzカーネル) |
| 更新 | Forrest-Tomlin (U-eta列 + R-eta行) | Forrest-Tomlin (`updateFT`) — **構造が違う** |
| 求解 (FTRAN/BTRAN) | FTRAN-L は GP疎求解、**BTRAN-L は転置行major + 密度ゲート付き scatter (§2.6 で実装)**、U は密スキャン | **L/U 両方向で hyper-sparse + sparse の切替** |
| 再分解 | 前回の**列順**を再利用 (`factorize_reusing`、行は選び直し) + フル Markowitz フォールバック (§2.2 で実装) | `refactor_info_` に前回ピボット順を記憶し `rebuild()` で再利用 (**ホットスタート時のみ**、§2.2) |

---

## 2. HiGHS 側で参考になる具体的な点

### 2.1 `buildSimple()` — 単位列・シングルトンの安価な剥離 (最重要)

HiGHS は本格的な Markowitz カーネル (`buildKernel`) に入る前に
`buildSimple()` で以下を O(nnz) の単純スキャンで処理する:

- **論理列 (スラック)**: 単に `u_pivot_value=1` を記録
- **単位列** (`count==1 && value==1`): 同上
- **行シングルトン**: その行の非ゼロが1つ → pivot決定後、他要素は全て L or U へ
- **列シングルトン**: 同様

残りだけ (`nwork` 本のベクトル) を Markowitz カーネルに回す。
ENOMOTO も `find_best_pivot` のバケットスキャンで「スコア0」を早期発見
(`PROF_TRIVIAL_STEPS` で90-100%がこれ) してはいるが、これは**カーネル内の
逐次探索**であり、HiGHS のように「記録なしの一括剥離」ではない。

> ENOMOTO の `lu.rs` コメント自身が「HiGHS の `buildSimple()` と同じ安価な
> 恩恵をバケットベース `find_best_pivot` が既に得ている」と主張しているが、
> HiGHS の剥離は Markowitz カーネルを**起動しない**点で異なる。
> 大規模問題ではカーネル起動時のデータ構造構築 (両方向CSR、リンク配列)
> そのものがコストなので、剥離率が高ければ起動を避ける価値はある。

**参考度: 高** — カーネルデータ構造の構築コストを丸ごと省ける可能性。

### 2.2 `refactor_info_` / `rebuild()` — 前回ピボット列の再利用

HiGHS は分解成功時に `refactor_info_` へ
`pivot_row / pivot_var / pivot_type` を保存する (`buildKernel` 内で push)。
`HFactor::build()` はまず `rebuild()` を試み、
**記憶したピボット順で `ftranL` を回すだけで L/U を再構築**する。
`buildSimple`+`buildKernel` のピボット探索全体をスキップできる。

> **訂正 (2026-09-22)**: この節はもともと「反復中に基底がほぼ同じなのだから
> HiGHS 方式は探索コストを大幅に削れる」と書いていたが、**HiGHS が
> `rebuild()` を使うのは反復中の再分解ではない**。`refactor_info_.use` を
> 立てるのは `HEkk::setNlaRefactorInfo()` だけで、これはホットスタート
> (保存した基底へ戻って解き直す)経路である。反復中の再分解
> (`HSimplexNla::invert()`)は `refactor_info_.clear()` 済みの状態で
> `buildSimple`+`buildKernel` を通る。`rebuild()` が相対安定性判定を持たず
> 絶対値 `pivot_tolerance` だけを見て、外れたら即 rank deficiency を返すのも
> 「同じ基底を分解し直すだけだから外れないはず」という前提による
> (`assert(abs_pivot >= pivot_tolerance);`)。

**実装済み (2026-09-22、`analysis/pivot_order_reuse_20260922_120602.md`)**:
上記のとおり (行, 列) 両方の順をそのまま再生する HiGHS 形の移植は、
基底が変わっている以上ほぼ必ず棄却される(NETLIB 実測で採択率 0〜4%、
棄却理由のほぼ全部が「記録した行に成分がない」)。FT 更新は基底スロットの
列を差し替えるが、入基底列が「出た列のピボット行」に非ゼロを持つ理由は
ないからである。

そこで `sparse_lu::factorize_reusing` は **列順だけを再利用**し、行は
毎ステップ選び直す(記録した行が安定性床を通ればそれを使い、だめなら
「床を通る候補のうち未到達列に残る非ゼロ数が最小」= 列固定下の Markowitz
カウントの残り半分で選ぶ)。分解本体は左向き (Gilbert-Peierls 形) で、
探索も能動部分行列 (`MarkowitzState` の `BTreeMap`/`BTreeSet`) も持たない。
安全弁は 3 つ (特異 / fill 上限 1.25 倍 / border 列基底の除外) で、いずれも
フル Markowitz へのフォールバックに落ちる。棄却は連続しやすいので指数
バックオフ付き。

結果: 再分解フェーズ自体が 25〜40% 減り、NETLIB93 全問題の同一プロセス
A/B で合計 **-2.55%**(46.42s → 45.24s)、目的関数値 93/93 一致、
**10% 以上の退行ゼロ**。fill 上限を緩めると採択率は上がるが総和では負ける
(2.0 倍で +2.7%)ことも計測済み — fill は 1 回限りのコストではなく、その
分解が生きている間の全 FTRAN/BTRAN が払い続けるため。

### 2.3 カーネル分解の両方向一体型データ構造

ENOMOTO は `BTreeMap`/`BTreeSet` のツリーノード走査で次数と要素を管理。
HiGHS は:

- `mc_*` (列major) と `mr_*` (行major) を**並行して持つ**
- 次数は別配列 `mc_count_a`/`mr_count`
- ピボット候補は**リンクリスト** (`col_link_first[count]`) で次数昇順に辿る
  (バケットと目的は同じだが、リンク付け替えで O(1))

ENOMOTO の `eliminate` は `BTreeSet`/`BTreeMap` の `entry()` と
`remove`/`insert` で O(log d) の木走査を繰り返す。コメントにも
「elimination の内側ループが最ホット」と書かれている。
HiGHS方式のフラット `Vec<HighsInt>` + `Vec<f64>` + 挿抜インデックスは
キャッシュ局所性で有利 (下記 §3.1 参照)。

**参考度: 中〜高** — ただし大規模リファクタリングが必要。

### 2.4 `pivot_threshold` と `colFixMax` のインクリメンタル更新

**実装して計測、両方とも不採用 (2026-09-22)**。詳細は
`analysis/pivot_threshold_colfixmax_20260922_154500.md`。

HiGHS は `pivot_threshold` (デフォルト0.1、`kMinPivotThreshold=8e-4`〜
`kMaxPivotThreshold=0.5`) を可変にでき、数値的失敗のたびに
`kPivotThresholdChangeFactor=5.0` 倍して上げる。`colFixMax` は
**列の最大絶対値を O(col_count) で更新**し、安定性基準を
`mc_min_pivot[j] = max_value * pivot_threshold` としてキャッシュする。

1. **安定性床の動的化 — 計測 +7.4%、不採用 (機構のみ温存)**。
   HiGHS と同じ向き (数値的失敗で**きつくする**) で実装した。閾値はスレッド
   ローカルに解ごとに持ち、分解ごとに1回だけ読む。FT更新の棄却・`x_B(M)`
   ドリフト・`d` ドリフト・PRICEとのピボット不一致の4トリガが 10 回たまる
   ごとに 0.25 → 0.5 へ1段上げる。
   NETLIB93 合計 **+7.4%**、10% 以上の退行が 3 問題
   (`pilot87` +29.2%、`brandy` +67%、`gfrd-pnc` +12.3%)。
   上限 0.5 が重い問題には高すぎる (`STABILITY` 自身のドキュメントが記録
   している「静的 0.5 は充填増で約4%遅い」がそのまま出る) のに対し、
   ドリフト起因の再分解だけで 10 回に届く問題が多く (`pilot` 21、`dfl001` 24、
   `greenbea` 22)、「病的な解にだけ効く安全弁」にならなかった。
   段幅を上げると NETLIB93 では一度も発火しなくなるだけなので、再調整では
   なく **既定で無効** (`extended_dual::PIVOT_ESCALATION_STEP = 0`) とし、
   配管と env ゲート (`ENOMOTO_PIVOT_THRESHOLD` /
   `ENOMOTO_PIVOT_ESCALATION_STEP`) だけ残した。既定ビルドの挙動は変更前と
   同一。
   なお本節が元々書いていた「悪条件時に**緩める**」向きは、`STABILITY` の
   ドキュメントが既に測っている (0.1 では `pilot` のドリフト起因再分解が
   88 → 190 回)。

2. **`colFixMax` のインクリメンタル化 — 計測 +2.8%、不採用**。
   `eliminate` のマージ内で「増えた/減った」を直接反映し、最大値が下がり
   得るときだけ stale にする版を実装した。`max` に丸めが無いため
   ピボット選択は変更前と**ビット単位で同一** (`ENOMOTO_VERIFY_COL_MAX=1`
   で clean 列も再走査して一致を assert、10 問題で確認) で、差分は実装
   コストのみ。それでも NETLIB93 合計 **+2.8%**。
   理由は §2.5 の `PIVOT_SEARCH_LIMIT = 8` が先に入っていること:
   `ensure_col_max_abs` は `find_best_pivot` が実際に見る候補列 (1ステップ
   最大8列、実測平均 4〜6列) でしか呼ばれないので、削れる再走査が既に小さい。
   `pilot87` で削減 150 万エントリに対し、マージループへ足す per-entry 更新
   (`m` 長配列への散在 read-modify-write) が 6150 万回。30〜40 倍の負け。
   本節が前提にしていた「`refresh_column` が毎回全列を再計算」は、既存の
   `col_max_abs_dirty` 遅延再計算と §2.5 の探索打ち切りにより、着手時点で
   既に成り立っていなかった。

**参考度: 低 (実測済み)** — 現在の `find_best_pivot` の形では、この2点に
残っている余地は無い。§2.5 の探索打ち切りを外す/緩める方向の変更を入れる
なら、2. は再検討の価値がある (分母が戻るため)。

### 2.5 `searchLimit = 8` によるピボット探索の明示的打ち切り

HiGHS `buildKernel`:

```cpp
HighsInt searchLimit = min(nwork, HighsInt{8});
...
if (searchCount++ >= searchLimit && merit_pivot < merit_limit)
  foundPivot = true;
```

つまり「8候補調べてそれなりの pivot が見つかれば即打ち切り」。
ENOMOTO の `find_best_pivot` は「score <= deg_col^2」という
**degree基準**の早期終了のみで、候補数の明示的上限はない。

HiGHS方式は最悪ケース (悪条件・密ステップ) の探索爆発を防ぐ。
ENOMOTO コメントでも「find_best_pivot のバケットスキャンは
非有界な場合がある」と認めている。

**実装済み (2026-09-22)**。ただし**上限は 8 ではなく 256**。
`analysis/pivot_search_limit_20260922_143000.md` 参照。

- 探索が実際に暴走していたのは `dfl001` (1ステップあたり平均 **261 候補列**、
  探索が solve 22.0s 中 **2.96s = 13%**) と `pilot87` (10.1s 中 1.23s)。
  「探索は全体の 0.2% 未満」という `lu.rs` の既存コメントは小規模4問題
  (`ganges`/`ship12s`/`stocfor2`/`fit1p`) での測定で、大きい問題には
  当てはまらなかった (訂正済み)。
- **HiGHS と同じ 8 は不採用**: 探索時間は全問題で減るが、93問題中 **64問題**で
  選ばれるピボットが変わり、分解の最終桁 → 双対比率テストのタイブレーク →
  反復数、と伝播する。向きは問題ごとに実質ランダム (`greenbeb` 反復数 +18%、
  `25fv47` −11%)。独立2回の全問題計測で合計は −4.5%/−3.7% と良いが、
  10% 超の退行が3問題 (`greenbeb` +21/+22%、`pilot` +15/+18%、
  `grow22` +13/+13%) 残り、採用ルールを満たさない。
- 閾値掃引に「ちょうど良い小さい値」は無い: `16` は `pilot87` を **2.93倍**に、
  `64` は 26問題の挙動を変える。単調でも連続でもない。
- **256 は「探索が実際に暴走している所でだけ発火する」値**。挙動が変わるのは
  7問題のみで、残り86問題は反復数・再分解回数まで従来と完全一致する。
  全93問題・独立2回で合計 −2.11%/−0.84%、10% 超の退行ゼロ、
  93/93 optimal・目的関数値一致。`512` も測ったが `wood1p` +10.6% で失格。

**教訓**: HiGHS の定数をそのまま採らないこと。`BTRAN_L_SCATTER_FRACTION`
(HiGHS 0.5 に対し実測 0.10) と同じ結論に、今度は 8 対 256 という
32倍の開きで到達した。カーネルのデータ構造 (BTreeMap/BTreeSet 対
フラット CSR、§3.1) が違えば1候補あたりの探索コストも違う。

### 2.6 FTRAN/BTRAN の hyper-sparse / sparse 切替 (`solveHyper`)

HiGHS `ftranL/ftranU/btranL/btranU` はいずれも:

```cpp
double current_density = 1.0 * rhs.count * inv_num_row;
const bool sparse_solve = rhs.count < 0 || current_density > kHyperCancel
                          || expected_density > kHyperXxx;
if (sparse_solve) { /* 密スタイルの for i in 0..num_row */ }
else { solveHyper(...); /* DFSによるreach set限定 */ }
```

- `expected_density` を **FTRANの直前** に「前回の結果から推定」して切替
  (`kRunningAverageMultiplier` の移動平均、`buildKernel`内)
- 4方向すべて (L/U × FTRAN/BTRAN) に GP疎求解あり
- L と U の**両方**に列major (`l_index`) と転置 (`lr_index`) を持つ
  → BTRAN の `L^-T` も hyper-sparse 可能

ENOMOTO は **L の FTRAN のみ GP疎求解** (`l_solve_sparse_into`)。
`l_solve_into` の BTRAN (`l_transpose_solve_into`) は密スキャン。
`u_solve_into` も密スキャン。

ENOMOTO のコメントは「BTRANの L^-T は列major構造がないので試みなかった」
「U の GP疎求解は測定で +10.4% 回帰だった」と明記。つまり:

- **U の GP疎求解**: ENOMOTO 自身が試して負けた → HiGHS の
  密/疎切替 (`kHyperFtranU=0.10`) も ENOMOTO の疎な基底では
  恩恵が出にくい可能性が高い。**ただし** HiGHS は `sparse_solve` 側でも
  「`rhs.index` を1回だけ密スキャンせず走査」する点が違う。
- **L^-T (BTRANのL段) の転置列major (`lr_*`) を持つ**: ENOMOTO が
  「試みなかった」方向。HiGHS は `lr_start/lr_index/lr_value` を持ち、
  `btranL` で hyper-sparse。**ENOMOTO の `l_transpose_solve_into` は
  密スキャンのまま**なので、ここは HiGHS に倣って
  転置L列major + `solveHyper` を試す価値あり。

**参考度: 中〜高** (BTRAN側の `lr_*` 相当)。

**実装済み (2026-09-22、`analysis/btran_l_transpose_20260922_113000.md`)**:
`LuFactors::l_row` が `L` の行major ミラー (HiGHS の `lr_start/lr_index/lr_value`
相当) を持ち、`l_transpose_solve_scatter_into` が BTRAN の `L^-T` を
gather から scatter に変える。`w[s]` が内側ループ唯一の乗数になるので
`w[s] == 0.0` で段まるごとスキップでき、FTRAN 側が既に持っていた零スキップが
BTRAN 側にも入る。格納はフラット (`FixedRows::from_transpose` の計数ソートで
直接構築するので、`l_col` のフラット化が回帰した「小さい `Vec` を作ってから
余計にコピーする」要因を踏まない)。

**密度ゲート `BTRAN_L_SCATTER_FRACTION = 0.10` が本体**。両形式は同じ `L` の
要素に触れ、違うのはアクセスの形だけ (gather = ランダムロード + レジスタ集約、
scatter = ランダムリードモディファイライト)。`w` が密だと 1 段も飛ばせず
scatter は純損になる。ゲート無しの全93問題 A/B が合計 **+3.0%**、10% 超の
退行 2 問題、一方で 25fv47 -17.7% / stocfor2 -17.6% という**同じ変更が
±17% の両方を出す**結果を示した。閾値 0.50 は 0.10 より明確に劣る
(+2.0%、10% 超の退行 9 問題) ので、密度 0.1〜0.5 の帯域はすでに scatter が
負ける領域。HiGHS の定数をそのまま採らず測って決めた。

結果: 正しい基準 (`l_row` の構築コストを base 側に入れない) での93問題合計は
独立2回で **-0.05%** と **-1.70%**。2回とも10%を超える問題は無し
(10% 超の顔ぶれが実行ごとに完全に入れ替わる)。一貫して動く問題は
25fv47 +9% / pilot +5〜7% / pilot87 -7% で、いずれも**反復数の変化**で説明がつく
(scatter の加算順序が最終桁を変え、双対比率テストのタイブレークがずれる)。

**U の GP疎求解は2度目の不採用**: HiGHS と同じ `kHyperFtranU = 0.10` 相当の
ゲートを付けた版を実装・計測したが、ゲート付き BTRAN 比 **+3.6%**、10% 超の
退行 7 問題。`u_seq` の走査量は実測で 78〜96% 削れていた (`avg_reach_frac`
0.043〜0.217) にもかかわらず遅い。**削っていたのは元々安い部分**で、DFS の
スタック操作・ソート・epoch 判定の合計が素直な逐次走査に勝てない。
ゲート無しの過去の試行 (+10.4%) と同じ結論にゲート付きでも到達した。

### 2.7 FTRAN結果の `expected_density` 移動平均による事前切替

HiGHS は `buildKernel` 内で FTRAN 結果の密度を移動平均
(`kRunningAverageMultiplier`) で追跡し、それを `ftranL/ftranU` に渡す。
これにより「次はどの求解モードか」を**結果を見る前に**判断できる。

ENOMOTO の `should_use_dense_solve(rhs_nnz)` は入力nnzのみで判断し、
`expected_density` 相当 (過去結果の密度) は持たない。
U段のetaが太ると失敗しがちなので、FTRAN前の入力密度だけでは
判定が甘い可能性。

**参考度: 中** — 観測ベースの切替を1つ追加するだけ。

**実装済み (2026-09-22、`analysis/ftran_density_gate_20260922_062832.md`)**:
`sparse_lu::FtranDensity` が呼び出し地点ごとに FTRAN 結果密度の移動平均
(HiGHS と同じ係数 0.05) を持ち、`FtLu::should_use_dense_solve_tracked` が
入力nnz判定に `EXPECTED_DENSE_FRACTION = 0.35` のゲートを OR で足す。
指摘どおりの状況が実在した: 入力側判定は93問題で一度も発火していない一方、
入基底列 FTRAN の結果密度は dfl001 0.655、pilot87 0.803、fit2p 0.999、
pilot 0.874。ゲートは 59/93 問題で発火し、全93問題で合計 -1%
(greenbeb -20%、wood1p -18%、pilot -12%、dfl001 -4%; 最大の退行は
pilot87 +6%、これは反復数 +6% に起因)。

### 2.8 `HVector` の packed index 表現

HiGHS は RHS を `HVector { array, index, count }` で持ち、
非ゼロの index リストを保持する。FTRAN/BTRAN はこの `index` を

- 密求解: `i in 0..num_row` の全走査
- 疎求解 (`solveHyper`): `rhs->index[i]` の非ゼロのみ走査

と使い分ける。ENOMOTO は `Vec<f64>` のみで、非ゼロ index は
呼び出し側が別途 `touched` 等で管理 (`combined_touched` 等)。
HiGHS方式の方が求解自身が「今どこが非ゼロか」を持つので、
`solveHyper` の list 構築が自然。

ENOMOTO は既に `solve_sparse_into(rhs_sparse)` で疎rhsを受ける形に
しているので、本質は同じ。

~~だが**BTRAN側 (`solve_transpose_into`) には疎入力版がない**。~~
**(解消済み)** `FtLu::solve_transpose_unit` / `solve_transpose_unit_capture`
を追加した。このクレートのBTRANの右辺はほぼ全てが単位ベクトル `e_i` で
(ピボット行 `rho_p`、DSE重み更新の `rho`、`DseState::from_basis` の m 本の
参照解、拡張法の `trial_row_ratio` と polish 側)、残る密な右辺は
`y = B^-T c_B` と `w = B^-T alpha` だけ。単位ベクトルに限れば非ゼロは
1個なので、

- 冒頭の置換 gather `scratch[s] = rhs[col_perm[s]]` (ステップごとの
  ランダムアクセス読み) → `fill(0.0)` + 1ストア
- `u_transpose_solve_into` 冒頭の「z の非ゼロを needed 集合に播く」O(m)
  走査 → そのステップ (`col_perm_inv[i]`) を直接 mark

と、O(m) のパス2本が消える。`solveHyper` 相当の一般の疎入力BTRAN
(reach集合のDFS) ではなく、右辺の形が分かっている場合の特殊化である点が
HiGHS とは異なるが、実際に出現する右辺はこちらでほぼ尽きている。

なお既存の `solve_transpose_unit_into` とは別物。あちらは更新前の `u_seq`
の順序に依存する接頭辞スキップなので `update_count() == 0` を要求する
(`from_basis` 専用)。新しい方は `needed` 集合の仕組みをそのまま使うので
Forrest-Tomlin 更新後も有効。

**参考度: 中** (単位ベクトル以外の疎rhs BTRANは依然未実装だが、
該当する呼び出しが `y`/`w` の2つしかなく、どちらも密)。

### 2.9 `updateFT` — U列eta + UR転置の二重構造

HiGHS の Forrest-Tomlin は:

- `u_*` (列eta) と `ur_*` (その転置) を**両方**保持
- 更新時、新旧のetaで触れた行/列を両側から削除 (`u_last_p[i_logic]--`)
- `pf_*` に R行列 (行eta) を格納

ENOMOTO の `FtLu` は `u_seq` (位置順リスト) + `r_etas` + `row_owners`。
HiGHS は flat 配列 `u_start/u_last_p/u_index/u_value` と
`ur_start/ur_lastp/ur_index/ur_value` を持つ。更新時の
「pivot行をUから削除」を `ur_*` (転置) 経由で高速化している。

ENOMOTO の `try_update` は `row_owners[p]` を使って行pを含むetaを
O(1) で見つける設計で、これは HiGHS の `ur_*` と同目的 (似た発想)。
ただし HiGHS は**flat配列 + 転置インデックス**、ENOMOTO は
`Vec<UEta>` + `Vec<Vec<usize>>` (row_owners)。

**参考度: 中** — 構造は違うが目的は同じ。ENOMOTOの方が既に工夫済み。

### 2.10 etaの格納形式 (Sparse/Dense) と `HybridVec::pack`

ENOMOTO は `HybridVec::Sparse | Dense` を `DENSE_ETA_FRACTION=0.4` で切替
(旧 `OffDiag` / `pack_off_diag`。疎/密ハイブリッドのベクトル表現として
`crate::sparse` に移した)。
HiGHS は常に `(index, value)` の疎形式 (`u_index/u_value`)。
つまり ENOMOTO は HiGHS を既に超えている (密eta最適化) 部分がある。
ここは逆に **ENOMOTO の方が進んでいる**。

---

## 3. HiGHS のデータレイアウトが有利な点 (キャッシュ)

### 3.1 flat `Vec` + インデックス vs `BTreeMap`/`BTreeSet`

~~ENOMOTO の `eliminate` は1要素ごとに `BTreeMap::entry` の木走査 +
`BTreeSet::remove/insert` を行う。要素はポインタ経由でヒープ上に散在。~~
(§3.1 実装済み、下記)
HiGHS は `mc_index/mc_value` の**連続配列**上で、`colInsert`/`colDelete` は
末尾swapのみ。キャッシュミスが桁違いに少ない。

ENOMOTO のコメントには「`l_col` を `FixedRows` にフラット化したら
**回帰した**」という記録があったが、これは L 因子 (求解時) の話であり、
**分解中のアクティブ部分行列**をフラット化した記録ではない。
HiGHSのカーネル部分行列のフラット化は別の話。

なお L 因子側のほうは、その後「後付けコピーではなく分解中に直接フラットに
構築する」版 (`CscBuilder`) で取り直したところ全93問 -1.58% の勝ちになった
(§4-5、`analysis/sparse_consolidation_lu_20260922_095906.md`)。ただし内訳は
重い10問 -1.77% / 軽い83問 +0.44% で、**小問題側では圧縮形のアクセスコストが
残る**。この非対称性は、動的なカーネル部分行列を置き換える際にも効いてくる
はず。

**参考度: 高** — ただし最大のリファクタリング。ENOMOTO 自身
「BTreeMap ベースは測定で最速」としているため、単純置換は
前述の `FixedRows` 回帰と同様に負ける可能性も。要注意。

**実装済み・採用 (2026-09-22、`analysis/kernel_flat_matrix_20260922_143000.md`)**:
`lu.rs` の `KernelMatrix` が `MarkowitzState` の
`Vec<BTreeMap<usize,f64>>`(行)+ `Vec<BTreeSet<usize>>`(列ミラー)を
置き換えた。レイアウトは HiGHS の `mc_start/mc_space/mc_count` +
`mr_start/mr_space/mr_count` と同型だが**軸が逆**で、このクレートの
elimination は行志向 (ピボット*行*を影響行に撒く) なので値が行major、
索引だけのミラーが列側 (`u32`) になる。

HiGHS の末尾swap無順序集合とは違い**両方の run を昇順に保つ**。これは
意図的な追加コストで (単桁長の連続 run に対する `copy_within`、それでも
置き換えた木走査より遥かに安い)、順序つき容器との**挙動の完全一致**を買う:
`find_best_pivot` は Markowitz スコア同点もピボット絶対値同点も
first-encountered で解決し、LP の基底行列は ±1 の完全同点だらけなので、
ミラーの順序を崩すと選ぶピボットが変わり、分解も反復数も変わってしまう
= before/after が何を計測しているのか分からなくなる。

同じ理由で `eliminate` は要素ごとの挿抜ではなく、影響行を
**自身の run とピボット行スナップショットのソート済みマージ1回**で
丸ごと書き直す (`set_row`)。「動的な部分行列は insert/remove を繰り返すので
単純置換では負ける」という上記の警告が外れたのはこの形のおかげで、
`BTreeMap` を素朴に `Vec` へ置き換えただけなら警告どおりだった可能性が高い。

計測 (NETLIB93 全93問、base = `a4e29bf`/`7289525` だけを revert したツリー、
searchLimit=8 は両アームに入れたまま、base→after 交互3周の中央値):
合計 **-11.39%** (42.11s → 37.32s)、重い10問 -12.25% / 軽い83問 -1.78%、
**10% 以上の退行ゼロ**。反復数・再分解回数・`pivot_search` の候補数まで
両アーム完全一致 (経路保存) なので、差はそのままデータ構造のコスト差である。
効果は `refactor` フェーズに集中 (pilot87: wall の 48% → 17%、4.9s → 0.95s、
これだけで全体差 -4.2s のほぼ全部)。`find_best_pivot` のバケット走査も
同じ候補数のまま pilot87 -69% / dfl001 -38% になった (候補ごとの
`rows[i].get(&j)` が木降下から `KernelMatrix::row_get` になったため)。

`L` のフラット化で残った「軽い問題では圧縮形のアクセスコストが残る」という
非対称性は、**カーネル側では出なかった**。軽い問題は再分解回数が 0〜3 回
(`kb2` は 0 回) で、そもそもこのコードを踏まないため。

### 3.2 リンクリストによる次数昇順走査

HiGHS: `col_link_first[count]` から `col_link_next[j]` を辿る。
パラメータは `count` で直接インデックスできる。
ENOMOTO: `Vec<VecDeque<usize>>` のバケット + `col_bucket_pos` で
O(1) 移動。**機能的には等価**。キャッシュ的には Deque の方が
要素が連続でなく Vec<VecDeque> の Vec-of-Vec になる分、やや不利か。
ただしリンクリストも `next` を辿るのでどっこい。

**参考度: 低** — 目的は同じ。

---

## 4. まとめ: 優先度付き提案

ENOMOTO の高速化に効きそうな順:

1. ~~**`refactor_info_` 相当のピボット順再利用 (`rebuild()`)** — §2.2~~
   **実装済み (2026-09-22)**。ただし HiGHS と同じものではない: HiGHS の
   `rebuild()` はホットスタート (同じ基底) 専用で、(行, 列) 両方の順を
   再生する移植は採択率 0〜4% だった。**列順だけ**を再利用し行は選び直す
   `factorize_reusing` を採用。NETLIB93 合計 -2.55% / -4.31% (独立2回)、
   10% 超の退行なし (`analysis/pivot_order_reuse_20260922_120602.md`)。

2. **`buildSimple()` 相当の単位列・シングルトン一括剥離** — §2.1
   - Markowitz カーネル (BTreeMap/BTreeSet 構築) の起動自体を避けられる。

3. ~~**`find_best_pivot` に候補数の明示的 searchLimit (≈8)** — §2.5~~
   **実装済み (2026-09-22、`2840f7b` + `d4046ca`)。ただし上限は 8 ではなく 256**
   (`ENOMOTO_PIVOT_SEARCH_LIMIT` で上書き可、`0` = 従来の無制限走査)。
   計測記録は `analysis/pivot_search_limit_20260922_143000.md`
   (`lu.rs` の `PIVOT_SEARCH_LIMIT` の docs が参照しているのはこれ。
   一時期リポジトリに存在せず「記録なし」と書かれていたが、本コミットで追加した)。
   - 基点 964d29e での NETLIB93 独立2回: 合計 −2.11%/−0.84%、10% 超の退行なし。
     効くのは実際に探索が暴走していた `dfl001` (探索 2.96s → 1.54s) のみで、
     他86問題は反復数・再分解回数まで挙動不変。
   - **カーネルをフラット配列化した現行 main で再計測しても結論は同じ**:
     独立2回で合計 −5.58%/−6.25%、10% 超の退行ゼロ (`analysis` の §7)。
     他が速くなった分 `dfl001` の探索の比重が上がり、改善幅はむしろ拡大した。
   - HiGHS と同じ `8` は**計測の結果 不採用**: 93問題中64問題でピボット列が変わり、
     独立2回とも `greenbeb` +21/+22%、`pilot` +15/+18%、`grow22` +13/+13%。
     現行カーネルでも `greenbeb` +12.4%、`stocfor1` +10.8% で同じ負け方をする。
   - 「実装が軽くリスクが小さい」という当初の見立ては**半分外れていた**。
     コードは確かに軽い (実質20行) が、上限が発火した問題ではピボット列が
     変わり、そこから反復数が ±20% 動く。リスクは実装量ではなく
     「何問題の挙動を変えるか」に比例する。`8` は64問題、`256` は7問題。

4. ~~**BTRAN の `L^-T` を転置列major + hyper-sparse 化** — §2.6, §2.7~~
   **実装済み (2026-09-22)**。`l_row` + 密度ゲート付き scatter。
   NETLIB93 合計 -0.05% / -1.70% (独立2回)、10% 超の退行なし。
   同時に試した U 側の到達集合限定 FTRAN は +3.6% で不採用 (§2.6)。

5. ~~**カーネル部分行列のフラット配列化** — §3.1~~ → **`L`・カーネル部分行列
   とも実施、採用** (`analysis/sparse_consolidation_lu_20260922_095906.md` §2.4、
   `analysis/kernel_flat_matrix_20260922_143000.md`)
   - 予想どおり「分解中に直接フラットに構築」する版なら勝った。`CscBuilder`
     で `LuFactors::l_col` を `crate::sparse::CscMat` に。このファイルの4つの
     分解はいずれも L の列を**昇順に**吐くので、計数パスすら要らず追記だけで
     済む。L 全体のアロケーションが m+1 個から2個になる。
   - 全93問3回ずつで **-1.58%**。ただし内訳は **重い10問 -1.77% / 軽い83問
     +0.44%** で、無条件の勝ちではない。軽い問題では `l_col[s]` が短く、
     圧縮形の「列アクセスごとに offsets を2回読む」コストが相対的に重い —
     初回の `FixedRows` 実験が回帰した理由のうち、後付けコピーをやめても
     消えない部分がここに残っている。
   - ~~**カーネル部分行列 (`MarkowitzState` の `Vec<BTreeMap>`/`Vec<BTreeSet>`)
     自体は未着手**~~ → **実施、採用** (2026-09-22、
     `analysis/kernel_flat_matrix_20260922_143000.md`)。`KernelMatrix`
     (行major の値 + 列major の索引ミラー、どちらも昇順維持の可変長 run)。
     「分解中に insert/remove を繰り返すから `CscBuilder` 形では置き換え
     られない」という当初の読みは、**挿抜の粒度を変える**ことで回避した:
     `eliminate` が影響行を1要素ずつ触るのをやめ、行まるごとのソート済み
     マージ + `set_row` 一括書き戻しにしたので、行側は「昇順に吐くだけ」に
     なる。列ミラーだけが真に動的だが、こちらは索引 (`u32`) のみで
     `partition_point` + `copy_within` で済む。
     全93問 base→after 交互3周の中央値で **-11.39%**、重い10問 -12.25% /
     軽い83問 -1.78%、**10% 以上の退行ゼロ**、反復数・目的関数値は
     両アーム完全一致。`L` のときのような小問題側の負け (+0.44%) は出ない。

6. ~~**ピボット安定性床の動的調整・`colFixMax` のインクリメンタル化** — §2.4~~
   **実装して計測、両方とも不採用 (2026-09-22)**
   (`analysis/pivot_threshold_colfixmax_20260922_154500.md`)。
   - 安定性床の動的化は NETLIB93 合計 +7.4%、10% 超の退行 3 問題
     (`pilot87` +29.2%)。機構は既定無効で温存、env で再現可能。
   - `colFixMax` のインクリメンタル化は +2.8%。ピボット選択はビット単位で
     同一なので、差分はまるごと実装オーバーヘッド。§2.5 の searchLimit=8 が
     先に入ったことで、削れる再走査が足す per-entry 更新の 1/30 以下しか
     残っていなかった。

7. **`FtLu::fill_count` の O(1) 化** — (HiGHS 比較外、このクレート固有)
   - 再分解トリガー(3)が毎反復読むのに `u_seq` 全体 (長さ m) を舐め直して
     いた。更新側で加減するだけで済む。
   - 全93問3回ずつで **-0.25%**、内訳は **軽い83問 -1.86% / 重い10問 -0.10%**
     と、上の 5 とちょうど相補的 (毎反復の固定費が支配的な小問題で効く)。
   - **実施、採用**。

逆に ENOMOTO が**既に HiGHS より進んでいる**点:

- eta の Sparse/Dense 切替 (`HybridVec::pack`, `DENSE_ETA_FRACTION`)
- 密入力の `faer` 全委譲 (`is_dense_input`)
- ボーダー列検出による Schur 補分解 (`factorize_bordered`)
- 4段の再分解トリガ (residual / pivot / eta-fill / max-updates) +
  `updateVerify` 相当 (trigger 5)

これらは HiGHS にはない ENOMOTO 独自の最適化なので維持推奨。
