# fuzz_tiny で HiGHS より悪い値を「最適」と答える seed の調査 (2026-11-03)

bench に使う環境変数: `ENOMOTO_MIP_FZ2_FIX=1` (試作、既定では使わない。値が `1` なら全部、`n` なら整数行の正規化だけ、`m` なら併合した整数列の上限だけ)

ブランチ `ccr-1beed81b-ex0uhq`。発端は analysis/uccase12_mem3_20261103.md §5 の 4. (`benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` が
既定で 13/400 HiGHS と食い違う)。並行のセッションのうち 1 が「いつから」、3 が良い値の seed と全体の分類を受け持つ。ここは悪い値の seed。

## まとめ

- **悪い値の seed は 5 件** (61・159・195・210・362)。HiGHS の解の整数列を丸めた点は、どれも右辺 `1 - 2e-9` の行を 2e-9 だけ破る
  (厳密には実行不能、許容誤差 1e-6 の内)。ENOMOTO 自身も違反 1e-9〜2e-9 の解を最適として返す (`is_feasible` は 1e-6) ので、
  ENOMOTO の基準 (許容誤差 1e-6) では HiGHS の点は実行可能で、ENOMOTO はそれを誤って捨てている。
- **どちらが正しいかの判定 (事実)**: 0-1 列の全列挙 (`brute.py`、連続列は 0 に置く) で、許容誤差 1e-6 の最良値 (tol) と厳密な最良値
  (exact) を出した。悪い 5 件では ENOMOTO の値は tol より悪く、exact とも一致しない (中途半端)。159・195・210・362 は HiGHS = tol。
  **61 は HiGHS も tol (-35.54) より悪い** (HiGHS -26.50、ENOMOTO -25.36)。良い値の seed (81・148・260・324・350) は ENOMOTO = tol で、
  HiGHS が tol より悪い (セッション 3 の範囲だが、判定は `eval_default.txt` に出ている)。
- **原因 (事実)**: 前処理の **整数行の正規化 (`normalize_integer_rows`、src/mip/mod.rs)** が右辺を `floor(u/g + 1e-9)` で丸める。
  幅 1e-9 の連続列が許容誤差の幅の列として固定されて消える (`fix_tiny_width_columns`) と、行 `x_a + x_b + y <= 1 - 2e-9` は
  整数列だけの行 `x_a + x_b <= 0.999999998` になり、正規化が右辺を `floor(0.999999999) = 0` に切り下げる。x_a = 1 (違反 2e-9、
  許容誤差の内) を厳密に切る。右辺 `1 - 1e-9` なら `floor(1.0) = 1` で残るので、**違反 1e-9 の点は通し、2e-9 の点は切る**
  不揃いになる (ENOMOTO の解の違反が 1e-9〜2e-9 なのに、HiGHS の点 (違反 2e-9) を切る理由)。
- **切り分け (事実)**: 環境変数で手順を切ると、`ENOMOTO_MIP_NO_COEF_TIGHTEN` (係数の強化と正規化を両方止める) で 61・195・210・362 が
  tol に一致、`NO_PRESOLVE`・`NO_PROBING` は一部だけ。`NO_TINY_WIDTH_FIX`・`NO_CUTS`・`NO_RESTART`・`NO_ROW_MERGE`・`NO_TRIVIAL`・
  `NO_IMPLINT` は効かない (下の表)。試作の部分 `n` (正規化の余裕を 1e-6) だけで 5 件すべてが tol に一致し、部分 `m`
  (併合した整数列の上限の丸め、同じ 1e-9 の形) は効かない。根のカット・木の刈り・再スタートではなく前処理で落ちている。
- **試作 `ENOMOTO_MIP_FZ2_FIX=1`**: 正規化の丸めの余裕を 1e-9 → 1e-6 (整数の許容誤差 `INT_ROUND_TOL` と同じ) にする。併合した整数列の上限の
  丸め (mod.rs の `ub.min(((bound - others) / a + 1e-9).floor())`) も同じにする (効き目は見えなかったが同じ形の穴)。
  - `fuzz_tiny.py 400 0`: HiGHS との食い違い 13 → 9。全列挙 (tol) との食い違いは 9 → 4、**tol より悪い値を最適と返すのは 5 → 0**。
    残る 4 件 (139・196・201・246) は差が 1e-6 程度で、連続列 (幅 1e-9、費用 -300 まで) の寄与 (全列挙は 0 に置く)。残る HiGHS との食い違い
    9 件は、6 件 (61・81・148・260・324・350) が HiGHS の側が tol より悪いもの、3 件 (139・201・246) が上の 1e-6 程度の差。
  - `scripts/mip_fuzz_vs_highs.py 300` (`ENOMOTO_MIP_REL_GAP=1e-9`): 既定・試作とも 0/300。
  - `cargo test --release --lib`: 既定・試作とも 333 件通る (§4)。

## 1. 悪い値の seed と判定 (`eval_default.txt`・`eval_fz2.txt`、`fz_eval.py`)

| seed | ENOMOTO 既定 | ENOMOTO 試作 | HiGHS | tol (1e-6) | exact |
|---:|---:|---:|---:|---:|---:|
| 61  | -25.36 | -35.54 | -26.50 | -35.54 | -16.32 |
| 159 | -18.57 | -23.36 | -23.36 | -23.36 | -18.57 |
| 195 | -14.02 | -20.53 | -20.53 | -20.53 | -9.61 |
| 210 | -45.19 | -54.94 | -54.94 | -54.94 | -41.90 |
| 362 | -28.92 | -35.10 | -35.10 | -35.10 | -13.51 |

- HiGHS の解の違反は 2e-9 (`cls_default_summary.txt`、`fuzz_tiny_cls.py 0 400`、過去の記録と同じ分類)。
- 既定の ENOMOTO は tol と exact の間の値を返す。違反 1e-9 は通して 2e-9 は切るため (上の原因)。
- 過去の記録 (benchmarks/equiv2_bug_20261022/fuzz_tiny_cls.txt、analysis/equiv2_bug_20261022.md §5) で「許容誤差の扱いの差 (推測、手順は追っていない)」
  とされていた「こちらが悪い」8 件と同じ種類 (HiGHS の点が 2e-9 だけ行を破る)。当時の 4・55・217・297 は今は食い違わず、362 が新しく出た。

seed 61 の例 (行 2 `x3 + x2 + y10 <= 1 - 2e-9`、y10 は幅 2e-9): HiGHS の解は x3 = 1 で違反 2e-9。tol の最良 (-35.54) はさらに
行 5 `x1 + y11 + y12 + y10 <= 1 - 1e-9` で x1 = 1 (違反 1e-9、y11 = 1e-9 で 2e-9) を使う。

## 2. 手順の切り分け (`ENOMOTO_MIP_LOG=1` と手順を切る環境変数)

| 環境変数 | 61 | 159 | 195 | 210 | 362 |
|---|---:|---:|---:|---:|---:|
| (既定) | -25.36 | -18.57 | -14.02 | -45.19 | -28.92 |
| `NO_PRESOLVE` | -35.54 | -18.57 | -20.53 | -54.94 | -35.1 |
| `NO_PROBING` | -35.54 | -23.36 | -14.02 | -54.94 | -35.1 |
| `NO_COEF_TIGHTEN` (正規化も止まる) | -35.54 | -18.57 | -20.53 | -54.94 | -35.1 |
| `NO_TINY_WIDTH_FIX`・`NO_CUTS`・`NO_RESTART`・`NO_ROW_MERGE`・`NO_TRIVIAL`・`NO_IMPLINT` | 既定と同じ | | | | |
| `FZ2_FIX=n` | -35.54 | -23.36 | -20.53 | -54.94 | -35.1 |
| `FZ2_FIX=m` | 既定と同じ | | | | |
| tol (正解) | -35.54 | -23.36 | -20.53 | -54.94 | -35.1 |

- `NO_PRESOLVE` でも 159 が直らないのは、前処理なしでもプロービングの後の前処理 (同じ正規化が走る。ログ `normalized 1 integer rows`) が
  ある経路のため (推測。`FZ2_FIX=n` で直るので、正規化が切っていることは事実)。
- ログでは 195 で最初の前処理から `normalized 1 integer rows`、61・362 ではプロービングの後の前処理で `normalized 1〜2 integer rows` が出る。

## 3. 試作 (src/mip/mod.rs)

- `fz2_fix(c)`: `ENOMOTO_MIP_FZ2_FIX` が `1` か文字 `c` を含むとき真。
- `normalize_integer_rows`: 余裕 `rt = 1e-6` (`n`)。`(u/g + rt).floor()`・`(l/g - rt).ceil()`。
- 併合した整数列の上限 (`ub.min(((bound - others) / a + rt).floor())`): `m`。
- 既定の動きは変えていない (環境変数なしでは 1e-9 のまま)。

## 4. テスト

- `cargo test --release --lib`: 既定 333 passed・0 failed (18 ignored)、`ENOMOTO_MIP_FZ2_FIX=1` でも 333 passed・0 failed。
- 試作の回帰テストは足していない (環境変数の値は呼び出し箇所ごとにキャッシュされるため、テストの中で切り替えられない)。

## 5. 残り・提案

- 試作を既定にするかは、MIPLIB の問題で前処理の結果 (正規化の数・根の LP) が変わるかを見てから決める。整数の許容誤差 (1e-6) で丸めるのは
  前処理の他の整数の丸め (`problem.rs` の `INT_ROUND_TOL`、`domain.rs` の `FEASTOL`) と揃えるだけで、変わるのは右辺が整数から 1e-9〜1e-6 だけ
  下にある行に限る。
- 他にも 1e-9 で整数に丸める所がある (`solver.rs` の `resolve_unbounded_root`、ヒューリスティクスの丸め)。前者は非有界の判定の切片問題、
  後者は解の候補を作るだけで、最適解を捨てる経路ではない (読んだだけ)。
- fuzz_tiny の判定は HiGHS でなく全列挙 (`fz_eval.py`) で見る方がよい: HiGHS も許容誤差の扱いで tol より悪い値を返す (61・81・148・260・324・350)。

## 再現手順

```sh
pip install -q highspy==1.15.1 maturin
maturin build --release -o /tmp/whl && pip install -q --force-reinstall --no-deps /tmp/whl/*.whl
D=analysis/fuzz_tiny_worse_20261103
python $D/fz_eval.py 400 0 2>/dev/null | grep -E "^[0-9]+ ours|^\{"                          # eval_default.txt
ENOMOTO_MIP_FZ2_FIX=1 python $D/fz_eval.py 400 0 2>/dev/null | grep -E "^[0-9]+ ours|^\{"    # eval_fz2.txt
python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 2>/dev/null | grep -E "^seed|^bad"  # fuzz_runs.txt
CARGO_TARGET_DIR=/tmp/tgt_test RUSTFLAGS="-C link-arg=-L/usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib cargo test --release --lib
```

- ファイル: `brute.py` (全列挙)、`fz_eval.py` (ENOMOTO・HiGHS・全列挙の比較)、`eval_default.txt`・`eval_fz2.txt` (その結果、400 問)、
  `cls_default_summary.txt` (`fuzz_tiny_cls.py 0 400` の既定の分類)、`fuzz_runs.txt` (`fuzz_tiny.py 400 0`・`mip_fuzz_vs_highs.py 300` の既定と試作)。
- ENOMOTO のログは標準出力に混ざるので、`grep` で結果の行だけを取る。
