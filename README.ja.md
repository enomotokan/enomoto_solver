# ENOMOTO-Solver

[English README](README.md)

ENOMOTO-Solver は，Python のモデリングインターフェースと Rust のコアからなる，線型計画(LP)・混合整数線型計画(MIP)のソルバーです．

LP の解法には**傾き・切片双対二段解法**を実装しています．これは，双対単体法のフェーズ1の一つである人工上下限法(双対問題に対する大M法)の M を，数値として固定せず記号的に扱う方法です．

- **局面A(傾き問題)**：計算なしに双対実行可能となる全スラック基底から出発し，無限の上下限を ±1 に，右辺を 0 に置き換えた問題を解きます．その最適値から，元の問題が有限な最適値を持ちうるかが分かります．
- **局面B(切片問題)**：局面Aの最終基底から，傾きを固定して得られる問題を通常の双対単体法で解きます．
- **後処理**：人工の境界に残った変数があれば，最適性を保ったまま主比検定で有限な境界へ移します．

これにより，**数値の M を選ばずに，双対単体法だけで**，LP が有限な最適値を持つか，非有界か，実行不可能かを判別し，有限な最適値を持つ場合は最適基底解を返します．

## 必要なもの

- Python 3.9 以降
- Rust のツールチェーン(`rustc`，`cargo`)
- [maturin](https://www.maturin.rs/) 1.5 以降(拡張モジュールのビルドに使用)
- ベンチマークを実行する場合のみ：[highspy](https://pypi.org/project/highspy/) と C コンパイラ(Netlib のデータの展開に使用)

## インストール

```sh
git clone https://github.com/enomotokan/enomoto_solver.git
cd enomoto_solver
python -m venv .venv
# Windows: .venv\Scripts\activate   /   Linux・macOS: source .venv/bin/activate
pip install maturin
maturin develop --release
```

必ず `--release` を付けてビルドしてください．デバッグビルドは何倍も遅くなります．

## 使い方

### モデルの定義と求解

```python
import math
from enomoto_solver import Model, Variable

M = Model()
x = Variable(float, 0, 10)          # 連続変数 0 <= x <= 10
y = Variable(float, 0, math.inf)    # 上下限は無限でもよい
M.set_objective(x + 2 * y, sense="maximize")
M.add_constraint(x + y <= 10)
M.add_constraint(x - y >= -4)

sol = M.solve()
print(sol.status, sol.objective)    # optimal 17.0
print(x.value, y.value)             # 3.0 7.0
```

- `Variable(型, 下限, 上限)` は，最後に作った `Model` に変数を作ります(`model=...` で明示することもできます)．型は `float`(連続)か `int`(整数)です．上下限には `±math.inf` を使えます．自由変数は `Variable(float, -math.inf, math.inf)` です．
- 線型式は `+`，`-`，スカラー倍で，制約は `<=`，`>=`，`==` で書きます．
- `set_objective(f, sense="minimize" | "maximize")` で目的関数を設定します(既定は最小化)．
- `solve()` は常に `Solution` を返し，実行不可能・非有界・求解失敗でも例外は送出しません．結果は `Solution.status` で判断します．`status` が `"optimal"` のときだけ `objective` に値が入り，各変数の値を `.value` で読めます．それ以外では `objective` は `None` で，`.value` を読むと `RuntimeError` になります．

### 結果の状態と，実行不可能・非有界な問題

`solve()` は `status`，`objective`，`node_limit_hit` を持つ `Solution` を返します．`status` は `"optimal"`，`"infeasible"`，`"unbounded"`，`"infeasible_or_unbounded"`，`"not_solved"` のいずれかです．

既定では，有限な最適値がないと分かった時点(局面Aの終了時)で止まり，`"infeasible_or_unbounded"` を返します．`distinguish_infeasible_unbounded=True` を渡すと局面Bまで進み，両者を区別します．

```python
U = Model()
z = Variable(float, -math.inf, math.inf)
U.set_objective(z)                  # z を最小化
U.add_constraint(z <= 5)

sol = U.solve(distinguish_infeasible_unbounded=True)
print(sol.status)                   # unbounded
```

### 整数変数

```python
K = Model()
items = [(Variable(int, 0, 1), value, weight) for value, weight in [(60, 10), (100, 20), (120, 30)]]
K.set_objective(sum(v * value for v, value, _ in items), sense="maximize")
K.add_constraint(sum(v * weight for v, _, weight in items) <= 50)
sol = K.solve()
print(sol.objective, [round(v.value) for v, _, _ in items])   # 220.0 [0, 1, 1]
```

整数変数を含むモデルは，上の LP 解法で緩和問題を解く深さ優先の分枝限定法で解きます．簡素な実装であり，専用の MIP ソルバーと競うことは想定していません．

## ベンチマーク結果

HiGHS 1.15.1，CLP 1.17.11，SoPlex 8.1.0 と 207 問の LP で比較しました(2026年9月29〜30日に計測)．各欄は「解けた問題数 / 求解時間のずらした幾何平均(秒)」です(10 秒ずらし．解けなかった問題は制限時間の 600 秒として計算)．太字は各行で最良の値です．

| 問題集合 | ENOMOTO | HiGHS | CLP | SoPlex |
|---|---|---|---|---|
| Netlib，有限の最適解あり(93問) | 93 / **0.152** | 93 / 0.166 | 93 / 0.164 | 92 / 0.741 |
| Kennington(16問) | 16 / **0.964** | 16 / 1.68 | 16 / 1.16 | 16 / 5.76 |
| Mittelmann LPopt(40問) | 12 / **377** | 11 / 405 | **18** / 393 | 7 / 503 |
| Netlib 実行不可能(29問) | 29 / **0.00597** | 29 / 0.0154 | 29 / 0.0743 | 28 / 1.53 |
| 実行不可能な問題の双対，非有界(29問) | 29 / 0.0588 | 29 / 0.335 | 28 / 2.01 | 29 / **0.0332** |
| 全体(207問) | 179 / **10.6** | 178 / 11.0 | **184** / 11.3 | 172 / 13.4 |

207問全体の幾何平均は，ENOMOTO 0.084 秒，HiGHS 0.183 秒，CLP 0.100 秒，SoPlex 0.146 秒です．ENOMOTO は誤った答えを1つも返しませんでした．局面Aで有限の最適解がないと検出した非有界な27問では，非有界か実行不可能かの区別に要した追加の時間は求解時間の2%程度でした．

- 計算機：AMD Ryzen 7 5700U のノート PC(8コア16スレッド，メモリ 16 GB)，WSL2 の Ubuntu 22.04(メモリ上限 12 GB)．ENOMOTO と HiGHS は16スレッド，CLP と SoPlex は逐次．時間制限とスレッド数以外はすべて既定の設定です．
- 時間は求解の呼び出しのみ(MPS ファイルの読み込みとモデルの構築は含まず，プリソルブとポストソルブは含む)で，3回の中央値です(300 秒以上かかった場合は1回)．1回ごとに新しいプロセスで解いています．
- Mittelmann は，plato.asu.edu から取得できる公開 44 問から，この計算機のメモリに載らない最大の4問(thk_48，L2CTA3D，dlr2，Dual2_5000)を除いた40問です．
- 全体の表，問題ごとの時間，生データは [benchmarks/paper/summary.md](benchmarks/paper/summary.md)，`per_problem.csv`，`results.json` にあります．再現の手順(Linux)は [scripts/paper_bench/README.md](scripts/paper_bench/README.md) を参照してください．

### Netlib での HiGHS との手軽な比較

Netlib の LP データには再配布を明示的に認めるライセンスがないため，このリポジトリには含めていません([docs/netlib-data.md](docs/netlib-data.md))．最初に一度だけ取得・展開します．

```sh
python scripts/setup_netlib_data.py          # ネットワーク接続と C コンパイラが必要
```

そのうえで HiGHS と比較します(`pip install highspy`)．

```sh
python -m enomoto_solver.benchmark_highs --max-vars 100000 --out netlib_results.csv
```

各問題は別々のサブプロセスで解きます．計測する時間は求解の呼び出しのみで(MPS ファイルの読み込みとモデルの構築は含まない)，どちらのソルバーもプリソルブとポストソルブを含みます．

## ディレクトリ構成

| パス | 内容 |
|---|---|
| `python/enomoto_solver/` | Python のモデリングインターフェース(`Model`，`Variable`，`Function`，`Constraint`)とベンチマークのスクリプト |
| `src/model.rs` | Python から呼ばれる入口 |
| `src/presolve.rs`，`src/presolve/` | プリソルブ(スケーリング，行・列の縮約，ポストソルブ) |
| `src/simplex.rs` | 標準形の構築，連結成分への分解，主単体法 |
| `src/simplex/slope_intercept_dual.rs` | 傾き・切片双対二段解法 |
| `src/simplex/lu.rs` | 疎 LU 分解と Forrest–Tomlin 更新 |
| `src/mip.rs` | 分枝限定法 |
| `src/params.rs` | 許容誤差と調整用パラメータ |
| `docs/improvement_history.md` | 実装の変更とその計測結果の記録 |
| `benchmarks/netlib_dev_results.csv` | 開発中に行った Netlib の計測結果(1行が1問・1回の計測．`source_file` 列が元の結果ファイル名) |
| `benchmarks/paper/` | 論文に載せた HiGHS・CLP・SoPlex との比較(207 問，制限 600 秒．`scripts/paper_bench/`) |
| `benchmarks/mittelmann_results.*` | 以前の Mittelmann LPopt ベンチマークの HiGHS との比較(2026-09-24，公開 44 問，制限 600 秒．`scripts/run_mittelmann_benchmark.py`，Linux 専用) |

## 論文

手法と数値実験は次の論文にまとめています．

榎本 観，「双対単体法のみによる線型計画問題の完全な判別：傾き・切片双対二段解法」(準備中)．

## 生成 AI の使用について

手法の着想と理論は著者によるものです．実装の大部分は，著者の設計と指示のもとで AI コーディングアシスタント(Anthropic 社の Claude)が行い，どの実装上の工夫を採用するかは著者が比較実験に基づいて判断しました．

## ライセンス

[MIT License](LICENSE) © 2026 Kan Enomoto
