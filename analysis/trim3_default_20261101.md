# trim3 を既定にする (2026-11-01)

- trim3 = RENS のやり直しを本当にノード上限まで進んだときだけにする (`ENOMOTO_MIP_RENS_MORE_NODES_REAL`)・やり直しを続きから継続する
  (`ENOMOTO_MIP_RENS_RESUME`)・深さ 3 の RENS をやめる (`ENOMOTO_MIP_RENS_MAX_DEPTH=2`)・深さ 2 の `RENS (LP before cuts)` をやめる
  (`ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH=1`)。
- 根拠: `halve_submip_20261030.md` の bench9・bench10 (58 問 4 回で SGM −2.4%・解けた数 +5、Mittelmann 2 回で差なし)。
  neos-1171448 の時間切れの率が上がるのは REAL の道の入れ替わりで、RESUME のせいではない (`neos1171448_20261101.md`)。

## 既定にした内容と戻し方

| 設定 | 新しい既定 | 元の振る舞いに戻す |
|---|---|---|
| `ENOMOTO_MIP_RENS_MORE_NODES_REAL` | 使う (`env_def!`) | `=0` (RESUME も場合分けを含むので、RESUME も `0` にする) |
| `ENOMOTO_MIP_RENS_RESUME` | 使う (`env_def!`) | `=0` |
| `ENOMOTO_MIP_RENS_MAX_DEPTH` | 2 | `=1000` (上限なし) |
| `ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH` | 1 | `=1000` (上限なし) |

- 4 つ全部を戻す並びは `analysis/halve_submip_20261030/run.py` の `"pretrim3"`。これからの記録の `"def"` は trim3 を含む新しい既定。
- コードは `src/mip/solver.rs` の `rens_more_nodes_real`・`rens_resume_on`・`RENS (LP before cuts)` の深さの判定、`src/mip/heuristics.rs` の `rens_depth_ok`。

## 確かめ

- `cargo test --release --lib` (`CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-C link-arg=-lpython3.13"`): 333 件通過。
- `check.py` (新しいビルドで、環境変数なし・trim3 を明示・元に戻す形を 1 回ずつ。`ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_VERBOSE=1`、出力 `check.out`):

| 問題 | 環境変数なし | trim3 を明示 | 元に戻す形 |
|---|---|---|---|
| binkar10_1 | 最適 7.3 秒、やり直しの省き 5、深さ 3 の RENS 0、深さ 2 の LP before cuts 0 | 7.3 秒、5・0・0 | 8.2 秒、省き 0・深さ 2 の LP before cuts 2 |
| mas76 | 最適 10.7 秒、省き 4、深さ 3 の RENS 0 | 12.6 秒、5・0 | 14.7 秒、省き 0・深さ 3 の RENS 12 |
| ran14x18-disj-8 | 時間切れ 3748 | 時間切れ 3748 | 時間切れ 3736 |

  環境変数なしの新しいビルドは trim3 を明示した形と同じ働き (やり直しを省く・深さ 3 と深さ 2 の LP before cuts の RENS を作らない) をし、
  戻す形ではそれらが元通り現れる。最適値はどれも同じ。
