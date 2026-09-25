# ENOMOTO-Solver 設計書

## 1. 目的とスコープ

- 問題の**入力インターフェース**を Python で提供する。
- 内部の**データ保持**(行列は CSR 形式)・**前処理**・**最適化アルゴリズム**は Rust で実装する。
- Python 側は `Model` / `Variable` / `Function` / `Constraint` の 4 クラスで、
  代数的な記法(`x + 2 * y`, `f <= 3` 等)による問題定義を可能にする。
- 対応する問題クラスは連立一次不等式・等式制約を持つ**線形計画(LP)**および
  **混合整数線形計画(MIP)**(整数・バイナリ変数)。

## 2. 全体アーキテクチャ

処理は上から下へ一方向に流れる。Python 側は式を組み立てるだけで、実際の行列・
線形代数は一切保持せず、すべて PyO3 FFI 経由で Rust 側 (`self._core`) に委譲する。

```
[Python]  問題入力インターフェース
  Model()
    │
    ├─ Variable(型, 下限, 上限)  … Function のサブクラス
    │     │  演算子 (+ - *)
    │     ▼
    │  Function  … coeffs: {変数index: 係数}, constant
    │     │  比較演算子 (<=, >=, ==)
    │     ▼
    │  Constraint
    │
    └─ Model.set_objective(Function) / add_constraint(Constraint) / solve()
          │
          ▼  PyO3 FFI ― 係数は Vec<(usize, f64)> として受け渡し
┌─────────────────────────────────────────────────────────
│ [Rust]  enomoto_core クレート
│
│  PyModel (model.rs)                … variables/objective/constraints を保持
│         │
│         │  整数/バイナリ変数がある場合 ─┐
│         │                              ▼
│         │                    mip::solve_mip (mip.rs) が solver::solve_lp を
│         │                    ノードごとに反復呼び出し(分枝限定法)
│         ▼
│  solver::solve_lp (solver.rs)      … simplex::solve_lp_dual を呼び、
│         │                             目的関数値を復元して SolveResult を返す
│         ▼
│  simplex::solve_lp_dual (simplex.rs) … 境界変数対応の**双対**改訂単体法
│         │                             (構造変数は必ず両側有限境界を持つため
│         │                              双対実行可能な出発点は常に構築できる)
│         │                             1) Markowitz ピボットの疎LU分解
│         │                             2) Forrest-Tomlin 更新 + 4トリガー
│         │                                再分解方針(simplex/lu.rs)
│         │                             3) 主:EXPAND法による退化対策、
│         │                                主/双対:最急辺規則によるプライシング
│         ▼
│  {"status", "objective", "x", "node_limit_hit"} が Python へ返る
└─────────────────────────────────────────────────────────
```

補助的な内部表現:

| モジュール       | 役割                                                                 |
|------------------|------------------------------------------------------------------------|
| `types.rs`       | `VariableData` / `LinearExpr` / `Objective` / `ConstraintRow` / `Status` |
| `simplex/lu.rs`   | Markowitz ピボットの疎LU分解 + Forrest-Tomlin インクリメンタル更新(`LuFactors`/`FtLu`)。§4.4 参照 |
| `simplex.rs` | 主・双対の境界変数付き改訂単体法本体。`Model.solve()` から実際に呼ばれる、本番の LP ソルバー。§4.4 参照 |

> **旧内点法(IP-PMM)経路は非アクティブ化**: `interior_point.rs` / `interior_point/qp.rs` / `interior_point/scaling.rs` /
> `interior_point/redundancy.rs` / `interior_point/propagate.rs` / `interior_point/kkt.rs`(Ruiz スケーリング・冗長制約
> 除去・制約伝播・AMD 順序付け込みの疎 KKT 系 LDLᵀ 求解を含む内点法一式)は、
> `solver.rs` から呼ばれなくなった(`solver::solve_lp` は現在
> `simplex::solve_lp_dual` を直接呼ぶ)。ファイル自体はクレートの
> module tree に残しており(`lib.rs` の `mod` 宣言はそのまま)、内点法に
> 戻したくなった場合のために保持している。同様に `csr.rs` / `preprocess.rs` /
> `simplex.rs`(さらに旧い、最初の 2 フェーズ単体法とその前処理・自前 CSR
> 実装)は module tree からも外し、ディスク上にのみ残置している(§7 参照)。

**責務分界**:
Python 側は「式をどう書けるか」(構文・型チェック・演算子オーバーロード)にのみ関与し、
行列そのものは一切保持しない。`Variable` を作った瞬間に Rust 側 `add_variable` が
呼ばれてインデックスが払い出され、そのインデックスが Function/Constraint が持つ
スパース係数辞書 `{index: coeff}` のキーになる。`set_objective` / `add_constraint` の
呼び出し時にはじめて、その辞書が `Vec<(usize, f64)>` として Rust 側に渡り、
Rust 内部の `Vec<VariableData>` / `Objective` / `Vec<ConstraintRow>` に格納される。

## 3. Python 側 API

### 3.1 Model

```python
M = Model()
```

- インスタンス化した瞬間に「カレントモデル」スタックに積まれる。以後、モデルを明示指定
  しない `Variable(...)` 呼び出しは最後に作られた(あるいは `with M:` で入った)モデルに
  紐づく。
- `M.set_objective(f, sense="minimize")`: `f` が `Function` 型でなければ `TypeError`。
- `M.add_constraint(g)`: `g` が `Constraint` 型でなければ `TypeError`。
- `M.solve() -> Solution`: Rust 側の前処理・最適化アルゴリズムを実行。
  最適解が得られたかどうかにかかわらず例外は送出せず、常に `Solution` を返す
  (HiGHS・Gurobi などと同じ流儀)。結果は `Solution.status` で判断する。
  `status` が `"optimal"` のときだけ `objective` に値が入り、各 `Variable.value` が
  読み出せるようになる(それ以外で読むと `RuntimeError`)。

### 3.2 Variable(Function のサブクラス)

```python
x = Variable(float, 0, 10)   # 型, 下限, 上限 ― float は連続変数
y = Variable(int, 0, 5)      # int は整数変数
z = Variable(int, 0, 1)      # バイナリ変数は [0, 1] に境界を絞った整数変数として表す
```

- `型` は組み込み型 `float`(連続) / `int`(整数)を第一引数に渡す。
  後方互換として文字列 `"continuous"` / `"integer"` / `"binary"`
  (`"c"`/`"i"`/`"b"` 等の別名含む)も引き続き受け付け、`"binary"` を使うと
  Rust コア側の専用バイナリ型(渡した下限・上限に関わらず常に `[0, 1]` に固定)になる。
- コンストラクタ内で `model._core.add_variable(vtype, lb, ub)` を呼び、Rust 内部の
  `Vec<VariableData>` に新しい行が追加され、その添字 `index` を受け取る。
- `下限`・`上限` は**どちらも有限な実数でなければならない**(`math.inf` /
  `-math.inf` を渡すと `add_variable` 側 [`model.rs`] が `ValueError` を送出する)。
  自由変数(境界なし)は表現できない ― §4.4 の「境界付き単体法」という
  不変条件を Python 側の入力段階で保証するための制約。
- `Variable` は `Function(model, {index: 1.0}, 0.0)` として自身を初期化するため、
  `Function` が持つ演算子(`+`, `-`, `*`, 比較)がそのまま使える。

### 3.3 Function

```python
f = x + 2 * y
```

- 内部表現は `coeffs: Dict[int, float]`(変数インデックス→係数)と `constant: float` のみ。
- `int` / `float` / `Variable` / `Function` 同士の `+`, `-`, `*`(スカラー倍のみ、
  非線形になる `Function * Function` は非対応)、単項 `-` を定義。
- `__radd__ = __add__`, `__rmul__ = __mul__` により `2 * y` のような左からの
  スカラー演算も同じ経路で処理される。

### 3.4 Constraint

```python
g = 0 <= f          # 0.__le__(f) は NotImplemented → f.__ge__(0) に反転して呼ばれる
g = f >= 3
g = f == 10
```

- `Function.__le__` / `__ge__` / `__eq__` が `Constraint(model, coeffs, sense, rhs)` を返す。
  `sense` は `"<="` / `">="` / `"=="` のいずれか一つのみ(2 方向の範囲制約はチェーン
  比較の意味論上サポートしない仕様どおり)。
- Python の比較演算子の反転規則により、`0 <= f` のようにスカラーを左に書いても
  `Function.__ge__` が呼ばれるため、両方向の書き方が自然に成立する。
- `Constraint` は `Function` を継承しない独立クラス。`Model.add_constraint` は
  `isinstance(g, Constraint)` で厳密にチェックする。

## 4. Rust 側の内部設計

> **§4.1〜4.3 は非アクティブな経路の記述**: `solver::solve_lp` は現在
> `simplex::solve_lp_dual`(§4.4)を直接呼び、以下に説明する
> CSR/QP標準形/IP-PMM の一式(`interior_point/kkt.rs`, `interior_point/qp.rs`, `interior_point.rs`, `interior_point/scaling.rs`,
> `interior_point/redundancy.rs`, `interior_point/propagate.rs`)は呼ばれなくなっている。ファイル・記述
> ともに、内点法へ戻す場合や参考実装として残してある。

### 4.1 CSR 行列 (`interior_point/kkt.rs`)

```rust
pub type Csr = faer::sparse::SparseRowMat<usize, f64>;

pub fn csr_from_rows(rows: &[Vec<(usize, f64)>], n_cols: usize) -> Csr;   // 三つ組 → CSR
pub fn mat_vec(mat: &Csr, x: &[f64]) -> Vec<f64>;                        // A x
pub fn mat_t_vec(mat: &Csr, n_cols: usize, y: &[f64]) -> Vec<f64>;       // Aᵀ y
```

`csr_from_rows` が、モデル構築中に貯めていたスパース行のリスト(トリプレット形式)
を CSR に変換する。目的関数・制約行列はここに格納される(`interior_point/qp.rs` が `A`(等式)・
`G`(不等式+変数境界)を組み立てる際の型として使う)。

> **CSR のまま faer に渡せるか?** できる。faer は `SparseRowMat`(CSR)と
> `SparseColMat`(CSC)の両方をネイティブに持ち、どちらも三つ組
> `(row, col, value)` からの一括構築(`try_new_from_triplets`)と、要素ごとの
> 参照(`.get(row, col)`)に対応している。これは「制約を行ごとに蓄積し、`solve()`
> 時にまとめて構築する」という本プロジェクトのアクセスパターンにそのまま合致する
> ため、自前の `CsrMatrix` 実装(旧 `csr.rs`)は廃止し、faer 純正の `SparseRowMat`
> を直接使うように置き換えた。`A`/`G` は生成されてから `interior_point.rs` の残差計算・KKT
> 組み立てに渡るまで一度も密行列に変換されない。
>
> ただし faer の疎コレスキー系ソルバー(§4.3)は**上三角のみを格納した CSC**
> (`SparseColMat`)を要求するため、KKT 行列の組み立てだけは `A`/`G` の CSR 行を
> 読みながら CSC の三つ組を新たに作る変換が入る(行持ち⇔列持ちの違いであって、
> 疎⇔密の変換ではない)。`A`/`G` 自体を CSC で持ち替える必要はない。

### 4.2 QP 標準形への組み立て (`interior_point/qp.rs`)

PIQP が扱う QP 標準形

```
minimize    ½ xᵀ P x + cᵀ x
subject to  A x = b
            G x <= h
```

に、モデルの変数境界・制約をそのまま変換する。単体法のときのような
「`x >= 0` に変数を置き換える」前処理(シフト/分割)は不要になったため、
一旦省略した:

- 目的関数の線形項 → `c`(このプロジェクトでは `P = 0` の LP のみ対応)
- `==` 制約 → `A`/`b` の行として追加
- `<=` 制約 → `G`/`h` の行としてそのまま追加
- `>=` 制約 → 両辺を `-1` 倍して `<=` に正規化してから追加
- 変数の上限・下限 → `x_j <= ub_j`, `-x_j <= -lb_j` という `G`/`h` の行として追加
  (有限な境界のみ。無限側は行を追加しない)

### 4.3 最適化アルゴリズム (`interior_point.rs`, `mip.rs`)

- **LP**: [PIQP 論文](https://arxiv.org/abs/2304.00290)(Schwan, Jiang, Kuhn, Jones,
  *"PIQP: A Proximal Interior-Point Quadratic Programming Solver"*, CDC 2023)の
  Algorithm 1 をゼロから再実装した内点法。「Infeasible Interior Point Method」と
  「Proximal Method of Multipliers」を組み合わせ、Mehrotra の予測子・修正子法で
  1 ステップずつ更新する:
  1. **予測(affine)ステップ**: 相補性条件を無視した Newton 方向を計算。
  2. **中心化パラメータ** `σ_k = clamp(μ_aff/μ, 0, 1)³` を予測ステップの結果から算出。
  3. **修正・中心化ステップ**: 相補性の 2 次項と中心化項を加えた右辺で再度 Newton
     方向を計算し、fraction-to-boundary 則(`τ = 0.995`)でステップ幅を決めて更新。
  4. 近接正則化パラメータ `ρ`, `δ`(下限 `1e-10`)を、残差の改善状況に応じて
     Algorithm 2(論文)のルールで毎反復更新。
  - 各反復の Newton(KKT)系はスラック方向 `Δs` を消去した対称な準定符号
    (quasi-definite)行列になる(論文 Remark 1)。この行列を **faer** の
    汎用スパースコレスキー入口(`faer::sparse::linalg::cholesky::
    factorize_symbolic_cholesky` + `SymbolicCholesky::factorize_numeric_ldlt`、
    準定符号行列に対する「コレスキー分解族」の求解法)で解く(`interior_point/kkt.rs`)。
    この汎用 API は **simplicial**(列ごとに逐次処理、非常に疎な場合に有利)と
    **supernodal**(構造の似た列をまとめて BLAS3 的な密カーネルで処理、fill-in
    がある程度大きい場合に有利)のどちらのカーネルを使うかを、推定 flop 数の
    比率から**自動選択**する。実測では 1000 変数規模の KKT(次元約 3500)で
    実際に supernodal が選ばれることを確認したが、素朴な simplicial 実装
    (旧 interior_point/kkt.rs)からの切り替えで壁時計時間には有意な変化が出なかった ――
    このスケールではカーネル選択よりも §7 に挙げる他のオーバーヘッド
    (毎反復のトリプレット再構築・ヒープ確保・シングルスレッド実行など)が
    支配的だったことを示唆している。
  - **前処理**: `interior_point::solve` の冒頭で問題全体に対して 1 回だけ **Ruiz 均等化**
    (`interior_point/scaling.rs`)を適用し、`A`/`G`/`c` を列(変数)・行(制約)ごとに
    正負・大きさが揃うようスケーリングしてから内点法を走らせる
    (最終的な `x` は解いた後にスケールを戻す)。さらに KKT 行列に対しては
    **AMD**(近似最小次数順序付け)による fill-in 抑制の並べ替えと
    **シンボリック分解を 1 回だけ計算してキャッシュ**する(`SparseKkt` が
    `Option<SymbolicCholesky<usize>>` で保持し、初回の `solve()` 呼び出し時に
    遅延計算。並べ替え・カーネル選択とも `factorize_symbolic_cholesky` が
    内部で行う)。KKT 行列の非ゼロパターンは反復間で不変(対角の値だけが
    変わる)なので、2 回目以降の呼び出しは数値分解
    (`SymbolicCholesky::factorize_numeric_ldlt`)だけをやり直せばよく、
    並べ替えの適用・復元も呼び出し側で管理する必要がない(内部で完結する)。
  - **冗長な等式制約の除去**(`interior_point/redundancy.rs`、Ruiz 均等化の直後・内点法ループの前に
    1 回だけ実行): 2 段階のパスからなる。
    1. **直接重複検出**: 各行を先頭係数で正規化し(右辺も同じ係数で正規化)、
       ビットパターンをハッシュ比較して完全一致・スカラー倍の重複行を除去する
       (行列演算不要の軽量な前処理)。
    2. **ランク明示 QR**: 残った行を列ベクトルとする密行列
       `[Aᵀ; bᵀ]`(係数 n 個 + 右辺の計 n+1 行)を組み立て、
       `faer::linalg::solvers::ColPivQr`(列ピボット選択付き QR、`A^T P^T = QR`)
       でランクを判定する。`R` の対角成分が(先頭ピボットに対して相対的に)
       無視できるほど小さいピボット位置は、列置換 `P` を通じて元の行番号に
       逆引きし、その行を削除する。右辺 `b` を係数と同じ拡張ベクトルに含めて
       いるため、「係数だけは他の制約の線形結合だが右辺が矛盾する」行
       (実際には実行不可能を意味する)を誤って冗長行として消してしまうことはない
       ―― その場合は拡張ベクトルとしては線形独立になるため QR は正しくこの行を
       残し、後段の Farkas 証明ベースの実行不可能性判定に委ねられる。
    faer の疎 LU(`sparse::linalg::lu`)は正方行列限定で、かつ厳密なゼロピボット
    しか特異性として検知しない(浮動小数点的にほぼゼロの数値的縮退は素通りする)
    ため採用しなかった。等式制約数 `p` は変数数 `n` に比べて小さいことが多く、
    密な QR を 1 回だけ計算するコストは実用上問題にならない。
  - **不等式行の制約伝播**(`interior_point/propagate.rs`、等式制約の重複除去の直後に 2 回実行):
    Achterberg, Bixby, Gu, Rothberg, Weninger, *"Presolve Reductions in
    Mixed Integer Programming"*(ZIB Report 16-44)の §3.1/§3.2 に基づく。
    行の最小・最大アクティビティ `inf{Ai·x}`/`sup{Ai·x}`(変数境界から計算)を
    使い、`sup <= b` なら行を冗長として削除、`inf > b` なら即座に実行不可能と
    判定して内点法を起動せずに返す(§3.1)。それ以外の行では、注目変数 `x_k`
    を除いた活性化下限 `l_iS` を使って `x_k` の境界を締め付ける(§3.2)。
    この実装では変数境界自体が `interior_point/qp.rs` によって `G`/`h` の単変数行として
    折り込まれているため、まずそれらを明示的な `lb`/`ub` として取り出し、
    残った多変数行に対して伝播を回したのち、締め付け後の `lb`/`ub` から
    単変数行を再構築して `G`/`h` を組み立て直す。整数変数向けの
    切り上げ・切り下げ(論文の該当箇所)は `interior_point.rs` の時点で変数型情報が
    渡っていないため未実装。
  - **実行不可能性・非有界性の判定**: 論文の HSD(自己双対埋め込み)方式ではなく、
    正則化パラメータ `ρ`, `δ` が下限に達し残差が改善しなくなった時点で
    Farkas の補題に基づく証明を試みる。`(y, z)` を正規化して
    `AᵀY + GᵀZ ≈ 0` かつ `bᵀy + hᵀz > 0` が成り立てば実行不可能、
    `x` を正規化して `Ax ≈ 0`, `Gx <= 0`, `cᵀx < 0` が成り立てば非有界と判定する。
    どちらの証明も成立しない場合は、反復上限(既定 100)まで進めたのち、
    相対的に小さい方の残差で暫定的に分類するフォールバックを用いる。
- **MIP**: 整数・バイナリ変数が 1 つでもあれば分枝限定法(`mip.rs`)を使う。
  各ノードは変数境界を上書きした LP 緩和を `interior_point::solve` で(ウォームスタートなしで)
  再度解き、分数値に最も近い 0.5 を持つ整数/バイナリ変数で `floor` / `ceil` に
  分枝する深さ優先探索。ノード数上限(既定 20,000)に達した場合は
  `node_limit_hit=True` とともにその時点の最良解を返す。内点法は厳密に整数値の
  頂点には到達しない(常にわずかに内部の点)ため、整数解が確定した葉ノードでは
  整数/バイナリ型の変数の値を最も近い整数に丸めてから記録する。

### 4.4 単体法エンジン(`simplex.rs`, `simplex/lu.rs`)— `Model.solve()` の実行経路

`solver::solve_lp` が実際に呼ぶ、境界制約付き変数を扱う**主・双対改訂単体法**。
段階的に実装:

> **並列化(rayon)**: 各反復内の「非基底列(あるいは基底行)ごとに独立した
> スコア計算 → 最良のものを1つ選ぶ」という形のループ(主のDantzig/最急辺
> エントリング規則、Harris/EXPAND比検定の候補収集、双対の chuzr/chuzc、
> `SteepestEdgeState`/`DseState` の重み更新)はすべて `rayon` の
> `par_iter()`/`par_iter_mut()` で並列化済み。一方 `simplex/lu.rs` の
> 前進・後退代入(`LuFactors::solve`/`solve_transpose`, `FtLu` の各 solve)は
> 定義上ステップ `s` の結果が `s+1` 以降に使われる逐次依存があるため
> 並列化していない(疎な三角行列解を並列化するには段階スケジューリング等の
> より高度な手法が必要で、今回はスコープ外と判断)。`simplex::lu::factorize`
> (Markowitzピボット選択)内のループも並列化を検討したが、FT更新により
> 呼び出し頻度自体が低く、対象の行列サイズもこのプロジェクトの規模では
> 小さいため、スレッド調停コストが見合わないと判断し見送った。

> **境界付き単体法という不変条件**: 構造変数(ユーザーが `Variable` で作る
> もの)は下限・上限が**必ず有限な実数**であることを前提にしている
> (`model.rs::add_variable` が §3.2 の通り検証)。この前提のもとでは、
> ①自由変数(両側非有界)は表現できないため `NbStatus` に「Free」の場合分けは
> 不要(削除済み)、②`Tableau::crash_dual_feasible` は理論上必ず成功する
> (自由変数がある場合のみ失敗しうる仕組みだったため)、③目的関数は必ず
> 有界な実行可能領域上の線形関数になるので理論上 `Status::Unbounded` は
> 到達不能(コード上は将来のバグに備えた安全網としてのみ残置)。
> なお内部標準形が導入する**スラック変数**(`<=`/`>=` 行1本につき1つ)は
> この対象外で、`Le`/`Ge` 制約のスラックは引き続き片側非有界
> (`[0, inf)`)のまま扱う(比検定・EXPAND法側の無限判定ロジックはスラック
> のために残っている)。

- **Stage 1**(`simplex.rs`): 2フェーズ改訂単体法の骨格。フェーズ1は
  「基底変数の境界逸脱量の合計」を目的関数とする複合コスト法、フェーズ2は
  通常のDantzigルール。基底行列 `B` の求解は当初 faer の密 `PartialPivLu` を
  毎反復再分解するプレースホルダーだった。
- **Stage 2**(`simplex/lu.rs` 追加): `B` の求解を疎行列ベースに置き換えた。
  - `factorize()`: **Markowitz ピボット**による疎LU分解。数値安定性より
    疎性(fill-inの少なさ)を優先し、`(行の非ゼロ数-1)×(列の非ゼロ数-1)` を
    最小化する候補の中から、閾値ピボット(列内最大絶対値の10%以上)を満たす
    ものを選ぶ。faerには本手法(動的な行/列連結リストが必要)のsparse LUが
    ないため自前実装。`O(m·nnz)` で候補を再スキャンする素朴な実装(疎行列を
    行ごとの `HashMap` で保持)で、連結リストによる高速化はしていない。
  - `FtLu`: **Forrest-Tomlin 更新**による `B` のインクリメンタル更新。
    Forrest, J.J.H. and Tomlin, J.A., "Updated triangular factors of the
    basis to maintain sparsity in the product form simplex method",
    Mathematical Programming 2 (1972) の手法を、Huangfu, Q. and Hall,
    J.A.J., "Novel update techniques for the revised simplex method"
    (ERGO-13-001, University of Edinburgh, 2013) の式(1)〜(13)に基づいて
    実装。列 `p` の置換を固定された `L` を通して `U` への「スパイク」に
    変換し(`ã_q = L⁻¹a_q`)、1本の行イータ `R⁻¹ = I - e_p rᵀ`
    (`r = -u_pp · ẽ_p`, `ẽ_p` は部分BTRAN)で三角性を回復する。`U` は
    ピボットスロットごとのイータ列として保持し、更新のたびに対象スロットの
    イータを列の末尾へ**移動**する(FTRAN/BTRANは生成順ではなくこの列順で
    処理する必要があることを、手計算で密行列と突き合わせて検証済み)。
    2回目以降の更新で `ã_q` を作る際は固定の `L` だけでなく、それまでに
    作成済みの `R` イータ(生成順)も通す必要がある
    (`B_{k-1} = L R_1...R_{k-1} U_{k-1}` であり `L U_{k-1}` ではないため)。
    この点は実装時に検証テストで誤りが見つかり修正した。
  - **4トリガー再分解方針**(ユーザー指定どおり実装、`simplex.rs`
    の定数群 `FT_*` 参照): (1) 5反復ごとに真の基底行列に対する残差
    `‖A_Bx_B-rhs‖` を確認、(2) `FtLu::try_update` 内でその場、新ピボットが
    小さすぎれば即再分解、(3) 同じ5反復ごとのタイミングでイータファイルの
    蓄積量(fill-in)が `4×m` を超えれば再分解、(4) 更新回数が100回を超えたら
    無条件に再分解。
  - 検証: `simplex/lu.rs` 内で1〜3回連続更新を完全な再分解と突き合わせるテスト、
    および `simplex.rs` 内で60変数規模のLPを IP-PMM(`interior_point::qp::build` +
    `interior_point::solve` を `solver.rs` を経由せず直接呼ぶテスト専用ヘルパー
    `solve_via_ipm`)の結果と突き合わせるテストを追加。後者では約30回の
    FT更新を経ても真の基底行列に対する残差が一貫してゼロを保つことを確認。
- **Stage 3**: 退化対策の **EXPAND法**(Gill, Murray, Saunders and Wright,
  "A practical anti-cycling procedure for linearly constrained
  optimization", Mathematical Programming 45 (1989) 437-474)、**主・双対の
  最急辺規則**(Forrest, J.J.H. and Goldfarb, D., "Steepest-edge simplex
  algorithms for linear programming", Mathematical Programming 57 (1992)
  341-374 と、Huangfu, Q. and Hall, J.A.J., "Parallelizing the dual revised
  simplex method", arXiv:1503.01889 §2.2 の実行可能実装)、および**双対単体法**
  本体を実装。
  - **EXPAND法**: Harris の2パス比検定(緩和した境界での第1パスで上限
    `α1` を決め、正確な境界での第2パスで `α1` 以下かつ最大ピボットの行を
    選ぶ)に、増加し続ける許容誤差 `δ_k`(`EXPAND_K` 反復ごとにリセット)
    による下限 `α_min = τ/|pivot|` を追加。最終ステップ幅が常に
    `max(exact, α_min) > 0` になることが、巡回を防ぐ直接の保証。退化ピボットでは
    リービング変数を厳密に境界へスナップせず(`δ_k` 以内のわずかな逸脱を許容)、
    `EXPAND_K` 反復ごとの `expand_reset_nonbasics` でまとめて是正する。
  - **主の最急辺規則**: `SteepestEdgeState`(`gamma[j]=‖B⁻¹A_j‖²`)。更新式は
    参考にした二次資料(`gamma_j+β_j(1+γ_t)-2β_jτ_j`、βが1乗)と自前導出
    (`Sherman-Morrison`をFtLuのイータ更新と同じ要領で適用、βの**2乗**が
    正しいという結論)が食い違ったため、実際に1回ピボットして更新後の重みを
    「新しい基底行列をゼロから再分解して直接計算した値」と突き合わせる
    テスト(`steepest_edge_weights_match_brute_force_recompute`)で実証的に
    決着をつけた(2乗が正しいと確認)。
  - **双対単体法・双対最急辺規則**: `DseState`(`w[i]=‖e_i^T B⁻¹‖²`)。更新式は
    上記 Huangfu-Hall 論文の式をそのまま自前導出でも再現でき(今回は
    一次資料と導出が完全一致)、`chuzr`(DSE重み付き実行不能性が最大の基底行を
    leaving row に選ぶ)→`chuzc`(境界変数向けに符号条件で候補を絞り、
    `|d_j/α_pj|` 最小の列を entering column に選ぶ、古典的な比検定であり
    Harris二段階法やBFRTはまだ未実装)→更新、という基本形。双対実行可能な
    出発点は `Tableau::crash_dual_feasible`(全スラック基底で `y=0` なので
    reduced cost=生コスト。符号に合う側の境界へ非基底変数を割り当てる)で
    作る。構造変数は両側とも有限境界を持つことが保証されている(上記の
    不変条件)ため、この初期化は理論上**必ず成功**する(任意の実数 `c` は
    `c>=-TOL` か `c<=TOL` の少なくとも一方を満たすので、対応する境界が
    常に使える)。旧実装では自由変数のケースだけが失敗しうる原因だったため、
    フォールバック機構自体を撤去した(履歴: 元は `try_crash_dual_feasible`
    という `bool` を返す関数で、失敗時は主単体法 `solve_lp` へフォールバック
    していた)。
  - **Model.solve() への接続**: `solver::solve_lp` が(旧 IP-PMM 経路に代えて)
    `simplex::solve_lp_dual` を直接呼ぶよう変更。`mip.rs` の分枝限定法も
    ノードごとの LP 緩和がこの経路を通るため、間接的に新エンジンを使う。
    Python 側テスト(`pytest`, `examples/smoke_test.py`)は全て新エンジン経由で
    再検証済み。旧 IP-PMM 経路(§4.1〜4.3)はモジュールツリーに残置(§2参照)。
- 未着手: 双対の Harris 2段階比検定・BFRT(bound-flipping ratio test)、
  双対版 EXPAND 相当の退化対策(現状は古典的な比検定のみ)。

### 4.5 `PyModel` (`model.rs`)

PyO3 の `#[pyclass]` として公開される、Rust 側の唯一のエントリポイント。

```rust
add_variable(vtype: &str, lb: f64, ub: f64) -> PyResult<usize>
set_objective(coeffs: Vec<(usize, f64)>, constant: f64, sense: &str) -> PyResult<()>
add_constraint(coeffs: Vec<(usize, f64)>, sense: &str, rhs: f64) -> PyResult<()>
solve() -> PyResult<dict>   # {"status", "objective", "x", "node_limit_hit"}
```

## 5. ビルドと配布

- ビルドツールは [`maturin`](https://www.maturin.rs/)(`pyproject.toml` の
  `[tool.maturin] module-name = "enomoto_solver._core"`)。
- `python/enomoto_solver/` が純 Python パッケージ、`src/` が Rust クレート
  (`Cargo.toml` の `[lib] name = "_core"`)で、ビルド後 `enomoto_solver._core` として
  読み込まれる。
- 開発時: `maturin develop --release`(venv 内で実行)。

```
ENOMOTO-Solver/
├── Cargo.toml                 # faer (疎行列LDLᵀ・AMD) / pyo3 / rayon に依存
├── pyproject.toml
├── src/                       # Rust: 内部データ保持・最適化アルゴリズム(機能ごとにディレクトリ分割)
│   ├── lib.rs                 # module tree の宣言 + 各クラスタの役割の要約
│   ├── types.rs               # 共有データ型(VariableData/LinearExpr/Status/...)
│   ├── model.rs                # PyO3 エントリポイント(唯一の入力検証境界)
│   ├── solver.rs               # トップレベルのLPディスパッチ(simplex::solve_lp_dualを呼ぶ薄いラッパー)
│   ├── mip.rs                  # 分枝限定法(整数/バイナリ変数がある場合。solver::solve_lpをノードごとに呼ぶ)
│   ├── simplex.rs              # ★ Model.solve() が実際に使う主・双対改訂単体法
│   ├── simplex/
│   │   └── lu.rs               # ★ Markowitz疎LU + Forrest-Tomlin更新(simplex.rsが使う)
│   ├── interior_point.rs       # [非アクティブ] 内点法(IP-PMM + Mehrotra予測子・修正子法)
│   ├── interior_point/
│   │   ├── qp.rs               # [非アクティブ] モデル→QP標準形
│   │   ├── scaling.rs          # [非アクティブ] Ruiz均等化
│   │   ├── redundancy.rs       # [非アクティブ] 等式制約の重複・線形従属行の除去
│   │   ├── propagate.rs        # [非アクティブ] 不等式行の制約伝播
│   │   └── kkt.rs              # [非アクティブ] KKT系の疎行列組み立て・AMD順序付け・
│   │                           #   シンボリック分解キャッシュ・LDLᵀ求解(旧spkkt.rs)
│   └── legacy/                 # [廃止・module treeから除外] 最初期の実装、上記いずれとも無関係
│       ├── csr.rs              #   旧自前CSR実装
│       ├── preprocess.rs       #   旧単体法用の標準形変換
│       └── simplex.rs          #   最初の2フェーズ単体法(simplex.rsの前身ではない、別実装)
├── python/enomoto_solver/     # Python: 問題入力インターフェース
│   ├── __init__.py
│   ├── model.py
│   ├── variable.py
│   ├── function.py
│   ├── constraint.py
│   └── types.py
├── tests/test_model.py
└── examples/smoke_test.py
```

## 6. エラーハンドリング方針

| 状況                                         | 例外                              |
|----------------------------------------------|-----------------------------------|
| `set_objective` に `Function` 以外を渡した    | `TypeError`                       |
| `add_constraint` に `Constraint` 以外を渡した | `TypeError`                       |
| 制約の右辺・左辺で `<=`/`>=`/`==` 以外        | (構文上発生しない。比較演算子経由でのみ `Constraint` を生成) |
| 異なる `Model` に属する変数同士を演算          | `ValueError`                      |
| `Variable` の下限・上限に無限大を渡した       | `ValueError`(Rust 側 `model.rs::add_variable`、PyO3 経由) |
| `solve()` が最適解を得られなかった(実行不可能・非有界・求解失敗) | 例外なし(`Solution.status` で判断) |
| 最適解が得られていない状態で `Variable.value` を読んだ | `RuntimeError` |
| 未知の変数型・不等号文字列                    | Rust 側で `ValueError`(PyO3 経由）|

## 7. 既知の制約(MVP スコープ)

### 7.1 現行の単体法エンジン(`simplex.rs`)についての制約

- 双対単体法の比検定は古典的な実装(`chuzr`/`chuzc` の基本形)にとどまり、
  Harris の2パス比検定・BFRT(bound-flipping ratio test)は未実装。主単体法
  側は EXPAND法(§4.4)で退化対策済みだが、双対側には相当する対策がまだない
  (現状は素朴な比検定のため、双対側で巡回する可能性を理論上は否定できない
  — テストでは遭遇していない)。
- 双対実行可能な出発点の初期化(`Tableau::crash_dual_feasible`)は、構造変数が
  必ず両側有限境界を持つという不変条件のもとで理論上必ず成功するため、以前
  存在した「失敗時に主単体法へフォールバックする」機構は撤去済み(§4.4参照)。
- 分枝限定法(`mip.rs`)は依然としてウォームスタートなし: 各ノードは
  `simplex::solve_lp_dual` をゼロから呼ぶため、Markowitz分解・
  ノード間での基底の再利用はない(1 回の `solve_lp_dual` 呼び出し内でのみ
  有効)。旧 IP-PMM 経路でも同様にノード間キャッシュはなかったため、
  この点は劣化ではない。
- `MAX_ITERS`(20,000)は固定の反復上限。理論上、非常に大規模な問題や
  病的な入力ではこの上限で「ベストエフォート」の `Optimal` を返してしまう
  可能性が残る(EXPAND法・DSE重みはこれを実務上起きにくくするための対策で
  あって、上限そのものをなくすものではない)。

### 7.2 旧 IP-PMM 経路(§4.1〜4.3、現在非アクティブ)についての記録

現在 `solver::solve_lp` からは呼ばれていないが、モジュール自体は残っており、
以下は内点法が現役だった時点で確認された事実として記録しておく。

- 目的関数・制約は線形のみ(`Function * Function` は非対応。`interior_point.rs` 自体は
  `P = 0` 固定で QP 一般化はしていない)。
- **CSR は `A`/`G` の格納形式として最後まで疎のまま使われる**(`interior_point/kkt.rs` の
  `SparseRowMat`)。KKT 行列の組み立て・分解だけは faer の疎コレスキー系
  ソルバーが要求する CSC(`SparseColMat`、上三角のみ格納)に変換するが、
  これは行持ち⇔列持ちの変換であって密行列への変換ではない。
- 精度検証: ランダム生成した LP を [HiGHS](https://highs.dev/)(`highspy`)と
  比較したところ、小規模(変数 5〜15)・大規模(変数 900〜1100)のいずれでも
  最適値は一致した(最大誤差はいずれも 1e-4 未満、大半は 1e-6 以下)。速度は
  HiGHS の成熟した実装(単体法ベース、プリソルブ・ウォームスタート・並列化込み)
  に対して、小規模で概ね同オーダー、変数 1000 規模で当初は約 13〜15 倍遅かった
  (Ruiz スケーリング・AMD 順序付け・シンボリック分解キャッシュを導入する前は
  約 10,500 倍遅かった)。supernodal カーネルへの自動切り替えは実際にこの
  スケールで発動することを確認したが、それ単体では速度改善は観測されなかった。
  その後、`interior_point.rs`(`Workspace`)・`interior_point/kkt.rs`(`Setup`)の反復ごとのヒープ確保を
  すべて初回一括確保+使い回しに置き換えたところ約 5.8 倍遅いまで縮まった。
  残る差はプリソルブの薄さ(冗長な**不等式**の除去は未実装)・シングルスレッド
  実行・分枝限定法でのウォームスタート不在など、他の要因が支配的と考えられる。
- 実行不可能性・非有界性の判定は Farkas 証明のヒューリスティックであり、
  PIQP 本家(や HSD 埋め込み系のソルバー)のような理論的に保証された
  埋め込みベースの判定ではない。反復上限に達しても証明が得られない場合は、
  主・双対残差の相対的な大小で暫定的に分類する(§4.3 参照)。
- 旧実装(`csr.rs` / `preprocess.rs` / `simplex.rs`: 自前 CSR・2 フェーズ単体法
  とその ">= 0" 標準形変換)は module tree から外して「いったん廃止」の状態。
  ファイルはビルド対象外のまま残置しており、コンパイルは通らない可能性がある
  (今後の変更で追随していない)。
- `interior_point/redundancy.rs` は**等式制約**の重複・線形従属行のみを除去する。**不等式**
  制約(`G`/`h`、変数境界の行を含む)側の冗長性検出は未実装。
- KKT 系まわりの反復あたりの確保は解消したが(上記)、`interior_point/kkt.rs::solve_into` は
  依然として毎回 `symbolic_base` を `.clone()` している(値の並べ替え API が
  所有権を要求するため)。行列の nnz に比例する小さなコピーで、以前の
  「毎回トリプレットをソートし直す」コストに比べれば軽微だが、完全な
  ゼロアロケーションではない。
