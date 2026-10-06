# 分枝切除法 (branch-and-cut) 実装提案

HiGHS v1.15.1 (`highs/mip/`、約 3 万行) と COIN-OR CBC / Cgl (master) のソースを読んだ結果と、
本リポジトリの現状の調査をもとにした実装計画。

---

## 1. 現状の整理

`src/mip.rs` (190 行) は深さ優先の素朴な分枝限定法で、各ノードで `solver::solve_lp` を最初から呼び直している。

| 項目 | 現状 | 問題 |
|---|---|---|
| ノード LP | 毎回 presolve → スケーリング → 双対単体法 (コールドスタート) | presolve の写像がノードごとに変わるので基底を引き継げない。計算時間の大半が無駄 |
| ノード選択 | DFS のみ | 下界と gap が計算できない |
| 分岐 | 最も小数的な変数 | 文献上、ランダム選択とほぼ同等の弱さ |
| 枝刈り | 緩和が `Optimal` 以外なら捨てる | `Unbounded` / `NotSolved` のノードを黙って捨てる (誤答の原因になる) |
| ノード上限 | 暫定解がないまま上限に達すると `Infeasible` を返す | 誤った状態を返す |
| 暫定解 | 丸めた x を実行可能性の確認なしで採用 | 誤答の原因になる |
| presolve | 整数性を一切考慮しない (`vtype` を参照する箇所がない) | MIP 全体に対する presolve としては使えない |

LP エンジン側に欠けているもの (`simplex.rs` / `slope_intercept_dual.rs`):

- **永続的なソルバー状態がない**: 基底・LU・DSE 重み・被約費用 `d` は関数の局所変数として捨てられる。
- **境界変更後の再最適化ができない**: `solve_slope_intercept_dual_from_basis` (`slope_intercept_dual.rs:813`) は基底の列集合しか受け取らず、DSE 重みを初期化し直す。
- **行の追加・削除 (カット) ができない**: `StdForm` と `CsrMat` / `CscMat` は不変。
- **出力されない情報**: 基底状態、双対値と被約費用、Farkas 光線。Farkas 光線は `infeasibility_certified` で計算はしているが返していない。
- **制御パラメータがない**: 目的値の打ち切り (cutoff)、利用者が指定する反復上限・時間上限。

すでにあって再利用できる部品:

- BTRAN 行 `BasisKernel::btran_row` (`basis_kernel.rs:164`)、`factorize_basis` (`basis_kernel.rs:286`)
- ラグランジュ下界 `lower_bound_from_basis` (`slope_intercept_dual.rs:921`)
- 協調キャンセル `cancel::with_token` (時間上限に使える)
- 活動量 (activity) による伝播の式 (`presolve/propagate.rs`)

---

## 2. HiGHS と CBC の比較と方針

| 観点 | HiGHS | CBC | 採用 |
|---|---|---|---|
| 全体構造 | `HighsSearch` (dive + backtrack)、`HighsDomain`、`HighsNodeQueue` に責務を分割 | 2.2 万行の `CbcModel.cpp` を大量のビットフラグで制御 | **HiGHS 型** |
| ノードの持ち方 | root からの境界変更スタック。基底は dive 中だけ親から継承し、キュー内のノードは持たない | 親との差分 (`CbcPartialNodeInfo`) を持ち、ノードごとに root からたどって再構築。子の LP は取り出した時に解く (遅延評価) | **HiGHS 型** (子はすぐ評価する) |
| 未処理ノードのキュー | 赤黒木 3 本 (best-bound / 0.5·lb + 0.5·estimate / suboptimal)。10 回に 1 回は best-bound を強制 | ヒープ 1 本。best-bound は毎回 O(n) で走査 | **HiGHS 型** (Rust では `BTreeSet` で書ける) |
| 境界伝播 | 行の活動量を差分更新し、変化が `capacityThreshold` を超えた行だけ再伝播。境界変更のたびに不動点まで回す | 弱い (`tightenBounds` を定期実行するのと probing のみ) | **HiGHS 型** (性能差の最大要因) |
| 分岐 | pseudocost + reliability (minrel = 8)、product score、inference / cutoff / conflict の副スコア、強分岐の反復予算は全体で管理 | pseudocost + reliability (numberBeforeTrust = 10)、`markHotStart` を使う強分岐、片側が不可能なら反対側に固定 | **HiGHS 型 + CBC の固定規則** |
| カット生成 | 汎用の `generateCut` (lifted cover と CMIR の良い方) を tableau 行の集約 (Gomory 相当)、path 集約、mod-k に適用 | Cgl の個別生成器 (Gomory、MIR2、Knapsack、FlowCover、TwoMIR、ZeroHalf、Clique、Probing) | **HiGHS 型**。生成器 1 つで多くのカットを賄える。CBC の Gomory の数値安全策 (`away`、条件数、dynamism) は取り入れる |
| カットプール | age、efficacy (viol / ‖a‖)、並列度 0.1 で除外、LP 内の age 上限 10 | slack が基底に入ったら削除するだけ | **HiGHS 型** |
| root カットループ | 停滞判定 (stall 3 回)、ラウンド数 2√log₂(木サイズ) | 目的値の履歴 7 回、最大 20 パス、生成器ごとの頻度を root での効果から決める | HiGHS 型。生成器ごとの頻度制御は CBC の考え方を簡略化して使う |
| ヒューリスティクス | trivial、Feasibility Jump、randomized rounding、RENS / RINS (sub-MIP)、feasibility pump、予算は LP 反復の 5% | FPump (root)、rounding、dive 系、RINS / RENS / DINS | 安価なものから順に入れる |
| その他 | 被約費用固定 (root の lurking bound)、conflict analysis、clique table、probing、restart、対称性 | 被約費用固定、`analyzeObjective` (目的係数の gcd から cutoff 増分を決める) | 後半フェーズで HiGHS 型を入れる |

---

## 3. 提案するモジュール構成

```
src/mip/
  mod.rs            solve_mip (入口)、MipResult、状態判定
  solver.rs         MipSolver: root 処理 → 探索ループ → 終了判定 (HighsMipSolver::run に相当)
  lp_relaxation.rs  LpRelaxation: 永続的な双対単体法の上に、境界の差分反映 (flush)、cutoff、
                    カット行の追加と削除、基底の保存と復元を載せる
  domain.rs         Domain: 境界変更スタック (理由つき)、backtrack、活動量の差分更新による伝播
  search.rs         Search: evaluate_node / branch / backtrack / dive / plunge
  node_queue.rs     NodeQueue: BTreeSet × 2 (lb 順、hybrid 順) + suboptimal 集合
  pseudocost.rs     Pseudocost: 上下別の平均、reliability、inference、cutoff
  branching.rs      reliability 分岐 + 強分岐
  heuristics/       trivial.rs, rounding.rs, fj.rs (Feasibility Jump), rens_rins.rs
  cuts/             pool.rs, generation.rs (lifted cover / CMIR), transformed_lp.rs,
                    tableau.rs (Gomory 相当), aggregator.rs
  redcost.rs        被約費用固定
  params.rs         許容誤差と各種上限 (mip_feasibility_tolerance = 1e-6, rel_gap = 1e-4, ...)
```

---

## 4. LP エンジン側に必要な拡張 (フェーズ 0、最重要)

ここが整わないと、上位の実装はどれも性能が出ない。

新しく `simplex::persistent::DualSimplexState` を作る (`slope_intercept_dual.rs` の局所状態を構造体に引き上げる)。

```rust
pub struct DualSimplexState { /* StdForm(可変 lb/ub, 行追加可), basis, nb_status, FtLu, dse_w, d, x_B ... */ }
impl DualSimplexState {
    fn new(lp: &MipLp) -> Self;                         // presolve なし、スケーリングのみ
    fn change_col_bounds(&mut self, cols: &[(usize, f64, f64)]); // 非基底の値と x_B を更新し、双対実行可能性を保つ
    fn add_rows(&mut self, rows: &[SparseRow]);          // slack を基底に入れ、LU を拡張または再分解
    fn delete_rows(&mut self, mask: &[bool]);            // basic な slack を持つ行だけ削除し、基底を詰める
    fn solve(&mut self, lim: &SolveLimits) -> LpStatus;  // Optimal | Infeasible | ObjectiveBound | IterLimit | TimeLimit | Unbounded
    fn get_basis(&self) -> Basis;  fn set_basis(&mut self, b: &Basis);   // 列・行ごとの status
    fn snapshot(&self) -> Iterate; fn restore(&mut self, it: Iterate);   // 強分岐用 (LU を含めて保存と復元)
    fn col_value / row_value / col_dual(=被約費用) / row_dual / objective;
    fn btran_unit_row(&mut self, i: usize, out: &mut SparseVec);         // tableau 行 (Gomory 用)
    fn dual_ray(&self) -> Option<SparseVec>;                             // 不可能性の証明
    fn dse_weights(&self) -> &[f64];
}
```

設計上の注意:

- **MIP 用 LP では LP presolve を切る**: `dualize`、`sifting`、連結成分分解、IPM との競争 (race) も使わない。使うのはスケーリングだけとし、元の空間との写像を固定する。presolve は MIP 全体に対して root で 1 回だけ行う (フェーズ 6)。
- **cutoff には真の費用で計算した下界を使う**: 双対単体法は費用を摂動して解くため、`lower_bound_from_basis` (真の `c` を使う) が cutoff を超えた時点で `ObjectiveBound` を返す。
- **境界変更後は primal の不可能性だけが生じる**: その状態から dual simplex の stage B を再開すればよい。変更前に非基底だった変数は新しい境界に移す。
- **ベンチマークで退行がないことを確認する**: 既存の LP ベンチマーク (Netlib / Mittelmann) を回し、LP 経路に退行がないことを見る。新しい構造体は既存の関数から呼ぶだけの形にし、ロジックを二重に持たない。

---

## 5. 段階的な実装計画

各フェーズの終わりに、MIPLIB 2017 の easy から選んだ小〜中規模の約 30 問 (MPS は `highspy` で読む。既存の `benchmark_highs.py` の流れを使う) で次の 2 点を測る。

- 正しさ: 最適値が HiGHS と一致すること
- 性能: 時間の幾何平均を HiGHS と比べる

| フェーズ | 内容 | 主な参照元 | 見積もり (Rust 行数) |
|---|---|---|---|
| **0** | `DualSimplexState` (§4)。単体テストでは、境界を変更して warm start した結果がコールドスタートと一致することを確かめる | HighsLpRelaxation、Osi の hot start | 1,000–1,500 |
| **1** | 木探索の骨格: `Domain` (変更スタックと backtrack のみ)、`Search` (dive + backtrack、基底の継承)、`NodeQueue` (best-bound / hybrid)、cutoff、gap、上限 (時間 / ノード / gap)、状態の正しい分類、暫定解の実行可能性確認。この時点の分岐は pseudocost のみ | HighsSearch、HighsNodeQueue | 1,500 |
| **2** | 境界伝播: 行活動量の差分更新、整数境界の丸め、連続変数は改善 30% 以上の場合だけ採用、`capacityThreshold` | HighsDomain | 1,200 |
| **3** | reliability 分岐: 強分岐 (`snapshot` / `restore`、全体反復予算)、product score、inference / cutoff の副スコア、片側が不可能なら反対側に固定 | HighsPseudocost、`selectBranchingCandidate`、CbcNode | 800 |
| **4** | 安価な主ヒューリスティクス: trivial、simple / randomized rounding、Feasibility Jump。目的が整数値をとる場合の cutoff 増分 | HighsPrimalHeuristics、HighsFeasibilityJump、CbcModel::analyzeObjective | 900 |
| **5** | root のカットループとカットプール: `generateCut` (lifted cover / CMIR)、TransformedLp (境界と VUB の代入)、tableau 分離器 (Gomory 相当)、age / efficacy / 並列度、stall 判定、basic なカット行の削除 | HighsCutGeneration、HighsTableauSeparator、HighsCutPool、HighsSeparation、CglGomory (安全策) | 2,500 |
| **6** | 被約費用固定 (root の lurking bound とノード)、整数を考慮した MIP presolve (既存 presolve に整数の扱いを追加して postsolve を整える)、RENS / RINS (再帰 MIP) | HighsRedcostFixing、HPresolve | 2,000 |
| **7** | conflict analysis (dual proof)、clique table、probing、path / mod-k 分離器、restart、対称性 | ConflictSet、HighsCliqueTable、HighsImplications | 4,000 以上 |

期待される効果は概ね **0 ≫ 2 ≈ 5 > 3 > 1 > 4 > 6 > 7** の順。ただし 1 は他のすべての前提になる。

---

## 6. 先に決めておきたいこと

1. **MIP では LP presolve を使わない方針でよいか**: 写像を固定するため。代わりに整数を考慮した MIP presolve を root で 1 回行う。
2. **並列化**: 最初は単一スレッドで作る。HiGHS の並列 worker は後から追加された機能で、骨格には不要。
3. **ベンチマーク集合**: MIPLIB 2017 の benchmark / easy から 30 問程度を `benchmarks/miplib/` に置く。容量が大きいのでリポジトリには置かず、取得スクリプトだけにする案。
4. **Python API の拡張**: `SolveResult` に `best_bound`、`mip_gap`、`node_count`、`TimeLimit` / `NodeLimit` の状態を追加し、`solve()` に `time_limit`、`mip_rel_gap`、`node_limit` を渡せるようにする。
