# Mittelmann 先生に渡すときの形 (入り口・ビルド・説明すること、2026-11-03)

- 土台: ブランチ `ccr-1beed81b-ex0uhq` の `925975ae` (canddet + p の今の既定)。時間の上限の超過は `overrun.md`。
- 先生の公開ページ (http://plato.asu.edu/bench.html・`ftp/milp.html`) は、この環境からは TLS の中継の証明書の鎖で読めなかった
  (curl・WebFetch とも失敗。`analysis/ipm_cost_perturb_20261102.md` にも plato.asu.edu に繋がらないとある)。検索でも条件の書かれたページは
  出てこなかったので、**下の計測条件は確かめていない**。起動例は時間の上限・スレッド数を差し替えて使う形にした。

## 1. 入り口があるか

| 入り口 | MPS を読む | 時間の上限 | スレッド数 | 相対ギャップ | 解のファイル | 終わりの要約 |
|---|---|---|---|---|---|---|
| `scripts/run_miplib_benchmark.py --worker-ours X.mps --time-limit T --threads N` | ○ (HiGHS で読む) | ○ (求解だけ) | ○ | × (0 固定) | × | JSON 1 行 (状態・目的値・下界・ノード・時間) |
| Python の `Model.solve(...)` (README) | × (モデルを組む API) | ○ | ○ | ○ | × | `Solution` |
| Rust の bin (`src/bin`・`[[bin]]`) | なし | | | | | |

- Rust の実行ファイルはない (クレートは `cdylib` の Python 拡張だけ)。MPS の読み込みも Rust 側にはなく、ベンチマークの台本はどれも highspy の
  `readModel` で読んで `benchmark_highs._build_our_model` でモデルを組んでいる。
- そこで最小の入り口 **`scripts/enomoto_mip_cli.py`** を別のファイルとして足した (既定の求解の経路は変えない。`--worker-ours` と同じく
  highspy で読み、`_build_our_model` で組み、`PyModel.solve` を呼ぶ):

```sh
python scripts/enomoto_mip_cli.py PROBLEM.mps.gz --time-limit 3600 --threads 8 --mip-rel-gap 1e-4 --solution PROBLEM.sol
```

  - `--time-limit` は既定で**起動からの壁時計**で数える (`--time-limit-scope total`): MPS の読み込みとモデルの組み立てにかかった秒を引いた残りを
    ソルバーに渡す。`--time-limit-scope solve` で求解だけに渡す (今までの計測と同じ)。
  - `--threads` を渡すと `ENOMOTO_MIP_THREADS` も同じ値にする (計測の台本 `run_worker` と同じ)。
  - `--solution`: 暫定解があれば MIPLIB の解の形 (`=obj= 値`、続けて `列の名前 値`) で書く。列の名前は MPS のもの。
  - 終わりに `status`・`objective`・`best_bound`・`gap`・`nodes`・`time_read`・`time_solve`・`time_total` を 1 行ずつ、最後に同じ内容の JSON を出す。
  - `--log` でソルバーの経過を出す。
- 確かめ: 10teams (`.mps.gz`、4 スレッド・60 秒) で `optimal 924`・解のファイル 2026 行を書き、highspy で読み直した問題に解を当てると
  行・境界の違反 1.1e-16・整数性の違反 0・目的値 924 で一致した。radiationm40-10-02 (33 MB の MPS) を上限 20 秒で回すと `time_read` 3.10 秒・
  `time_solve` 18.42 秒・`time_total` 21.58 秒 (外から測った壁時計 21.76 秒): 読み込みを引いた 16.9 秒をソルバーに渡し、ソルバーが 1.5 秒越えた
  (今の既定の超過、`overrun.md`)。

## 2. ビルド手順と必要なもの

- 必要なもの: Python 3.9 以上 (ここでは 3.13.18)、Rust のツールチェーン (ここでは rustc / cargo 1.97.0。`rust-toolchain` の指定はない)、
  maturin 1.5 以上 2 未満、C コンパイラ (`metis-sys` が同梱の METIS・GKlib を `cc` でコンパイルする。`bindgen`・libclang・cmake は要らない)、
  MPS を読むための highspy (ここでは 1.15.1)。numpy などほかの Python のパッケージは入り口には要らない。
- `.cargo/config.toml` が METIS の C のフラグ (`-fPIC -ftls-model=global-dynamic`) を置く。リポジトリの根で `cargo`・`maturin` を動かすこと
  (ほかの場所からだと効かず、拡張の読み込みが "cannot allocate memory in static TLS block" で失敗する)。

```sh
git clone https://github.com/enomotokan/enomoto_solver.git
cd enomoto_solver
git checkout <渡すコミット>
python -m venv .venv && . .venv/bin/activate
pip install "maturin>=1.5,<2" highspy==1.15.1
maturin build --release -o dist && pip install dist/enomoto_solver-*.whl   # または maturin develop --release
python scripts/enomoto_mip_cli.py X.mps.gz --time-limit 7200 --threads 8
```

- 必ず `--release` でビルドする (デバッグのビルドは何倍も遅い)。ビルドはこの機械 (4 CPU) で約 4 分。
- PyPI の `enomoto-solver` は 0.1.5 で、今の既定 (ブランチの先) より古い。**PyPI の wheel ではなくソースからビルドしてもらう**か、渡すコミットで
  wheel を作って渡す (`Cargo.toml` の version は 0.1.5 のまま。渡すなら上げてタグを打つのがよい)。
- MKL PARDISO は試験用の環境変数 (`ENOMOTO_T_CHOL_BACKEND=1`) のときだけ `libloading` で探す。既定では使わないので、先生の機械に MKL が
  あってもなくても振る舞いは同じ。
- 環境変数 `ENOMOTO_*` は計測では何も付けない (既定の形)。先生の環境に残っていないことを確かめてもらう。

## 3. ライセンス

- MIT (`LICENSE`、`Cargo.toml`・`pyproject.toml` とも `MIT`、著作者 Kan Enomoto)。
- 依存 (各 crate の `Cargo.toml` の license): faer 0.20・0.24 (MIT)、rayon (MIT OR Apache-2.0)、pyo3 (MIT OR Apache-2.0)、mimalloc・libmimalloc-sys (MIT)、
  metis-sys (MIT OR Apache-2.0、同梱の METIS・GKlib は Apache-2.0)、libloading (ISC)、num_cpus (MIT OR Apache-2.0)。MPS の読み込みに使う HiGHS (highspy) は MIT。どれも配布・計測に差し支えはない。

## 4. 先生に説明すること

1. **スレッド数**: `--threads N` (Python の API では `threads=N`)。既定は使える CPU の数 + 1 の半分 (HiGHS と同じ考え方)。木の探索・根の LP の
   競争 (双対単体法と内点法)・裏のカットのスレッド・FJ のスレッド・並列のサブ MIP がこの数を分け合う。`overrun.md` の計測では 4 スレッドの
   指定で CPU の使用は 1.4〜3.8 本分 (CPU 秒 ÷ 壁時計、時間切れの大きな問題は 1.5〜2.7) で、指定を超えて使う回はなかった。
   スレッド数による速さの違いは並列のセッションの記録 (`threads/`) を見る。
2. **時間の上限**: `--time-limit` 秒。ソルバーは期限で止まるが、大きな問題では 0.1〜数秒越えて返ることがある (`overrun.md`: 60 秒の上限で
   bench14〜20 の 6946 回中 54 回が 1 秒以上、今の既定に近い `def` は 2114 回中 18 回・最大 66.8 秒。上限 300 秒の radiationm40-10-02 は 304.7 秒)。
   試作 `ENOMOTO_MIP_OVERRUN3=zilhdr` (6 つの直し) で 60 秒の 1 秒以上の超過は 2 割前後 → 1 割未満 (残りは大きな根の LP の LU 分解で 1.1〜1.5 秒)、300 秒の超過は 4.7 秒 → 1.0 秒になった (既定にするかは bench の後で決める)。
   入り口は既定で起動からの壁時計で数え、MPS の読み込み (大きな問題で 2〜4 秒、highspy の読み込み + Python でのモデルの組み立て) も上限に含める。
   上限ちょうどで外から打ち切られる環境なら、`--time-limit` を数秒短めに渡すことを勧める (下の起動例)。
3. **ギャップ**: このソルバーの既定の相対ギャップは **0** (証明まで続ける)。HiGHS の既定は 1e-4、MIPLIB の「解けた」の判定も 1e-4 程度の
   ギャップを許す。先生の計測が各ソルバーの既定のままなら 0 のままでよいが、ほかのソルバーとそろえるなら `--mip-rel-gap 1e-4` を渡す。
   ギャップの定義は (上界 − 下界) ≤ G · max(|上界|, 1) (`src/mip/mod.rs`)。今までの記録 (bench1〜20・Mittelmann 186 問) はすべてギャップ 0。
4. **出力**: 状態は `optimal` / `infeasible` / `unbounded` / `infeasible_or_unbounded` / `time_limit` / `node_limit` / `not_solved`。
   `time_limit` でも暫定解があれば `objective` と解のファイルが出る。下界は `best_bound`。
5. **MPS の読み込み**: HiGHS (highspy) の `readModel` で読むので、HiGHS が読める形 (固定・自由形式、`.gz`) なら読める。読み込みの失敗は例外で止まる。
6. **メモリ (いちばん大きな注意)**: 長い上限では開いたノードの写しでメモリが大きく伸びる。radiationm40-10-02 は 300 秒で 11.7 GB、600 秒の途中
   (462 秒) で 14 GB に達して OOM (`overrun.md` の 5 節)。並列のセッションの記録 (`long_b.md`) でも uccase12 が 325 秒・13.6 GB で OOM、最大 RSS は
   HiGHS の 3〜21 倍。先生の機械のメモリ (と同時に何本回すか) を聞き、渡す前にメモリの上限の手当て (開いたノードの写しを捨てる・減らす) を入れる
   べき。今の形では、メモリの少ない機械で長い上限だと「落ちた」になる問題がある。
7. **既知の弱いところ**: 根の LP が大きい問題 (数十万の非零) では根の LP・前処理だけで時間を使い切ることがある。ノードを数えない回もある。
8. **乱数・決定性**: 同じ入力・同じスレッド数でも、並列の手順 (裏のカット・サブ MIP・木の分け合い) の進み方で結果と時間は揺れる
   (同じ問題を 3 回解いて 1 回だけ解けることがある)。

## 5. 起動例

- 先生の MILP の計測は、公開の記録の覚えでは MIPLIB 2017 のベンチマークの問題を数時間の上限・複数スレッドで回す形だが、上に書いたとおり
  条件はこの環境では確かめていない。上限 `T` 秒・スレッド `N` の例:

```sh
T=7200; N=8
for f in instances/*.mps.gz; do
  b=$(basename "$f" .mps.gz)
  python scripts/enomoto_mip_cli.py "$f" --time-limit $((T - 10)) --threads $N --solution "sol/$b.sol" > "log/$b.log" 2>&1
done
```

  - `T - 10`: 期限の後の後始末 (`overrun.md` の残り) と Python の終わりの分の余裕。今の既定では上限 300 秒で約 5 秒越える (木の待ち行列の片付け、
    `overrun.md` の 5 節)。7200 秒ではメモリが先に尽きないかぎり待ち行列はさらに大きくなるので、`ENOMOTO_MIP_OVERRUN3=zilhdr` (の既定化) を入れて
    から渡すのが安全。入れないなら余裕を 30 秒程度にする。
  - ギャップをそろえるなら `--mip-rel-gap 1e-4` を足す。
