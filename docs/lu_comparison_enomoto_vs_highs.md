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
| ピボット選択 | Markowitz score `(r-1)(c-1)` 最小 + 安定性床 | 同じ Markowitz、ただし **search limit = 8** の打ち切り |
| 前処理 | なし (バケットの早期終了のみ) | **`buildSimple()` で単位列・行列シングルトン・列シングルトンを O(nnz) で剥離** |
| 密行列dispatch | `is_dense_input()` → `faer::PartialPivLu` | なし (常にMarkowitzカーネル) |
| 更新 | Forrest-Tomlin (U-eta列 + R-eta行) | Forrest-Tomlin (`updateFT`) — **構造が違う** |
| 求解 (FTRAN/BTRAN) | L のみ GP疎求解、U は密スキャン | **L/U 両方向で hyper-sparse + sparse の切替** |
| 再分解 | 毎回ゼロから Markowitz | **`refactor_info_` に前回ピボット列を記憶し `rebuild()` で再利用** |

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
次回の同一基底分解時、`HFactor::build()` はまず `rebuild()` を試み、
**記憶したピボット順で `ftranL` を回すだけで L/U を再構築**する。
`buildSimple`+`buildKernel` のピボット探索全体をスキップできる。

ENOMOTO は毎回ゼロから Markowitz 探索。反復中に基底がほぼ同じ (数本入れ替え)
であることを考えると、HiGHS 方式は「探索コスト」を大幅に削れる。

> 注意: HiGHS は列順 (pivot順) の再利用が**数値的安定性の観点**でリスクにもなる
> ため、`rebuild()` が pivot tolerance を下回ったら rank deficiency を返して
> フル `buildSimple`+`buildKernel` にフォールバックする。この安全弁ごと移植
> する必要がある。

**参考度: 高** — ENOMOTO の分解は「衝突に強い BTreeMap 走査」が高コストなので、
探索スキップの効果は大きいと思われる。

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

ENOMOTO の安定性床は固定 `STABILITY = 0.1` で、`col_max_abs[j]` を
`refresh_column` で再計算する (BTreeSet全走査)。

HiGHS は `pivot_threshold` (デフォルト0.1、`kMinPivotThreshold=8e-4`〜
`kMaxPivotThreshold=0.5`) を可変にでき、`colFixMax` で
**列の最大絶対値を O(col_count) で更新**する。安定性基準は
`mc_min_pivot[j] = max_value * pivot_threshold` としてキャッシュ。

ENOMOTO の `refresh_column` は `col_rows[j]` 全体 (BTreeSet) を走査して
`max` を取る。HiGHS はカーネル内部でこれを列単位の連続メモリ走査にしている。

**参考度: 中** — `stability` を動的調整 (悪条件時に緩める) する余地。

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

**参考度: 高** — 実装が非常に軽い割に効く可能性 (探索上限を8程度に)。

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
しているので、本質は同じ。だが**BTRAN側 (`solve_transpose_into`) には
疎入力版がない**。

**参考度: 中**。

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

### 2.10 etaの格納形式 (Sparse/Dense) と `pack_off_diag`

ENOMOTO は `OffDiag::Sparse | Dense` を `DENSE_ETA_FRACTION=0.4` で切替。
HiGHS は常に `(index, value)` の疎形式 (`u_index/u_value`)。
つまり ENOMOTO は HiGHS を既に超えている (密eta最適化) 部分がある。
ここは逆に **ENOMOTO の方が進んでいる**。

---

## 3. HiGHS のデータレイアウトが有利な点 (キャッシュ)

### 3.1 flat `Vec` + インデックス vs `BTreeMap`/`BTreeSet`

ENOMOTO の `eliminate` は1要素ごとに `BTreeMap::entry` の木走査 +
`BTreeSet::remove/insert` を行う。要素はポインタ経由でヒープ上に散在。
HiGHS は `mc_index/mc_value` の**連続配列**上で、`colInsert`/`colDelete` は
末尾swapのみ。キャッシュミスが桁違いに少ない。

ENOMOTO のコメントには「`l_col` を `FixedRows` にフラット化したら
**回帰した**」という記録があるが、これは L 因子 (求解時) の話であり、
**分解中のアクティブ部分行列**をフラット化した記録ではない。
HiGHSのカーネル部分行列のフラット化は別の話。

**参考度: 高** — ただし最大のリファクタリング。ENOMOTO 自身
「BTreeMap ベースは測定で最速」としているため、単純置換は
前述の `FixedRows` 回帰と同様に負ける可能性も。要注意。

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

1. **`refactor_info_` 相当のピボット順再利用 (`rebuild()`)** — §2.2
   - 反復中は基底がほぼ同じ。探索をスキップできる効果は大きい。
   - ただし pivot tolerance を下回る時のフォールバック必須。

2. **`buildSimple()` 相当の単位列・シングルトン一括剥離** — §2.1
   - Markowitz カーネル (BTreeMap/BTreeSet 構築) の起動自体を避けられる。

3. **`find_best_pivot` に候補数の明示的 searchLimit (≈8)** — §2.5
   - 既に degree基準早期終了はあるが、最悪ケース対策として上限を追加。
   - 実装が軽くリスクが小さい。

4. **BTRAN の `L^-T` を転置列major + hyper-sparse 化** — §2.6, §2.7
   - ENOMOTO が「試みなかった」唯一の方向。HiGHS は `lr_*` を持つ。
   - `expected_density` の移動平均による事前切替も同時に追加候補。

5. **カーネル部分行列のフラット配列化** — §3.1
   - 理論的にはキャッシュ効率最大。ただし ENOMOTO 自身
     `FixedRows` 実験で回帰を経験しており、慎重に。
   - 「分解中の部分行列を直接フラットに構築」する版なら勝つ余地。

6. **ピボット安定性床の動的調整・`colFixMax` のインクリメンタル化** — §2.4
   - 悪条件問題での探索爆発・再分解を減らす。

逆に ENOMOTO が**既に HiGHS より進んでいる**点:

- eta の Sparse/Dense 切替 (`pack_off_diag`, `DENSE_ETA_FRACTION`)
- 密入力の `faer` 全委譲 (`is_dense_input`)
- ボーダー列検出による Schur 補分解 (`factorize_bordered`)
- 4段の再分解トリガ (residual / pivot / eta-fill / max-updates) +
  `updateVerify` 相当 (trigger 5)

これらは HiGHS にはない ENOMOTO 独自の最適化なので維持推奨。
