# 論文用ベンチマーク (ENOMOTO vs HiGHS / CLP / SoPlex)

Netlib (有限最適解あり 93 問、実行不能 29 問とその双対 29 問)、Kennington 16 問、Mittelmann LPopt の
公開問題で、ENOMOTO と HiGHS・CLP・SoPlex の求解時間を比べる。実行不能 29 問とその双対 29 問では、
ENOMOTO の段階 A・B の判定時刻も記録する。

## 手順 (Linux のクラウド環境、リポジトリのルートで)

```bash
# 1. ソルバーの用意 (CLP 1.17.11 と SoPlex 8.1.0 をソースからビルドし計時用ドライバを作る。
#    .paper_venv に highspy 1.15.1 と enomoto_solver を入れる。root 権限は不要)
bash scripts/paper_bench/setup_solvers.sh

# 2. 問題データの取得と展開 (ネットワークと gcc が要る。Mittelmann は圧縮のまま数 GB)
.paper_venv/bin/python scripts/paper_bench/prepare_data.py

# 3. 計測 (1 回の求解ごとに results.json に追記するので、中断しても同じコマンドで再開できる)
nohup .paper_venv/bin/python scripts/paper_bench/run_paper_bench.py > bench.log 2>&1 &

# 4. 表だけ作り直す (計測後に集計方法を変えたとき、複数台の結果を合わせるとき)
.paper_venv/bin/python scripts/paper_bench/run_paper_bench.py --report-only
.paper_venv/bin/python scripts/paper_bench/run_paper_bench.py --report-only --inputs m0/results.json m1/results.json
```

版は環境変数で変えられる (`CLP_VERSION`、`SOPLEX_VERSION`、`HIGHS_VERSION`。setup_solvers.sh の先頭を参照)。
結果は `benchmarks/paper/` に書く:

| ファイル | 中身 |
|---|---|
| `results.json` | 全計測の生データ (各回の時間・状態・目的関数値・反復数、ENOMOTO の節目の時刻、環境) |
| `summary.md` | 環境 (CPU の型番、ソルバーの版)、集合ごとの集計、ENOMOTO の段階別の時刻、揺らぎの目安、問題ごとの表 |
| `per_problem.csv` | 問題 × ソルバーごとの中央値・各回の時間・ENOMOTO の T_A/T_B |

## 計測の定義

- **時間**: 求解の呼び出しだけの実時間。MPS の読み込み (ENOMOTO はモデルの構築も) は含めない。
  ENOMOTO は `Model.solve()`、HiGHS は `Highs.run()`、CLP は `ClpSimplex::initialSolve()`、
  SoPlex は `SoPlex::optimize()` の前後 (いずれも前処理を含む)。
- **設定**: どのソルバーも既定値。時間制限だけ設定する (既定 3600 秒)。ENOMOTO は制限時間を持たない
  ので、親プロセスが制限時間で止める。ENOMOTO は実行不能と非有界を区別する設定
  (`distinguish_infeasible_unbounded=True`) で解く。
- **スレッド数**: 並列計算の効果も含めて測るため、ENOMOTO (rayon、`RAYON_NUM_THREADS`) と HiGHS
  (`threads` オプション) は既定で論理 CPU 数を使う。`--threads N` で変更、`--threads 0` で各ソルバーの
  既定値 (HiGHS の既定は論理 CPU 数の半分)。CLP・SoPlex は逐次のソルバー。HiGHS の双対単体法は
  `parallel` オプションの既定 (`choose`) では逐次で、スレッドを使うのは主に前処理や内部の一部。
- **繰り返し**: 1 回の求解ごとに新しいプロセスを起こし、各問題・各ソルバーを 3 回解いて中央値を取る。
  繰り返しの間でソルバーの順番を回す。1 回目が 600 秒 (`--single-run-above`) 以上かかった、または
  解けなかったときは 1 回だけ。1 回目が 1 秒 (`--warmup-below`) 未満なら、その回はウォームアップとして
  捨てて測り直す (新しい問題の初回だけ遅く出る現象があるため)。
- **解けたかどうか**: 期待どおりの状態 (最適 / 実行不能 / 非有界) を返し、最適なら目的関数値が基準値と
  相対 1e-6 以内。基準値は、最適と答えたソルバーの値のうち最も多くのソルバーと一致するもの。
  「実行不能または非有界」とだけ答えた場合は解けていない扱い。cplex2_dual は期待状態を定めない
  (下の注意を参照)。
- **集計**: 幾何平均、10 秒シフト付き幾何平均、総時間。いずれも解けなかった問題を制限時間として含める。
  あわせて全ソルバーが解けた問題だけの幾何平均も出す。
- **ENOMOTO の段階の時刻** (実行不能 29 問 + 双対 29 問、求解の開始 = 前処理の直前から):
  - T_A: 段階 A の終わりで z¹ < 0、つまり有限の最適解が無い (実行不能か非有界) と分かった時刻。
    段階 A を飛ばした問題 (どの列も M 側に置かれない)、z¹ = 0 だった問題、前処理が結論した問題には無い。
  - T_B: 段階 B の後で実行不能か非有界かを結論した時刻 (単体法の終了時刻。前処理が結論した問題は
    その時刻)。
  - 数値的破綻から解き直した場合は、最後の解き直し以降の節目を使う (時刻は最初からの経過)。
  - 節目は Rust 側の `src/phase_timing.rs` が記録し、`_core.last_solve_events()` で読む。
- **揺らぎの目安**: 3 回測れた問題について、(最大 − 最小) / 中央値 を時間帯ごとにまとめる。1 回しか
  測らない長い問題の揺らぎを直接見たいときは `--variability-probe 問題名 ...` (`--probe-reps`、既定 3)
  で追加計測する (中央値には使わず、別の表に出る)。

## 計測環境について

- 専有のインスタンス (専有ホスト、ベアメタルなど) を使い、計測中は他の処理を動かさない。計測は 1 問ずつ
  順に行う (並列に回すと時間が揺れる)。
- 可能なら CPU の周波数を固定する (`cpupower frequency-set -g performance`、ターボの無効化)。
  `summary.md` には CPU の型番、`lscpu` の出力、インスタンスの種類 (DMI)、CPU governor を記録する。
- 時間がかかるので、同じ種類のインスタンス複数台で `--shard i/n` (0 始まり) に分けて回し、
  `--report-only --inputs` で合わせられる。CPU の型番が違う結果を合わせると `summary.md` に警告が出る。
- 所要時間の目安 (見込み、未計測): Netlib・実行不能・双対・Kennington は合わせて 1 時間前後。
  Mittelmann は 1 問 × 1 ソルバーで最大 1 時間かかり、打ち切りが多いと全体で数十時間〜100 時間を超えうる。
- 1 プロセスのメモリ上限は既定で物理メモリの 90% (`--mem-gb`)。超えたら `m` (メモリ不足) と記録する。

## 注意

- cplex2_dual: cplex2 は「ほぼ実行可能」な問題で、双対は数値的には非有界にならない (HiGHS は最適値 0
  と答える)。期待状態を定めず、どの結論でも解けたとみなす。集計から除きたいときは
  `--report-only --exclude cplex2_dual` (計測時に `--exclude` を付ければ計測もしない)。
- Mittelmann の問題は `scripts/run_mittelmann_benchmark.py` の一覧 (plato.asu.edu から取れる公開問題)。
  rail02 など 5 問は plato.asu.edu に無いので含まない。
- 計測後に `ENOMOTO_*` の環境変数が設定されていると、`summary.md` の「環境変数の上書き」に出る
  (既定の設定で測るなら何も設定しないこと)。
