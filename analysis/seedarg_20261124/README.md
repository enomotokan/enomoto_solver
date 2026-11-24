# seedarg: Model.solve() に乱数の種の引数 seed を足す (2026-11-24、親の依頼・利用者の判断)

## 1. 中身

- `Model.solve(..., seed: Optional[int] = None)`。`log` の後、`*` (`root_solver`) の前に置いた。
  - `None` なら今まで通り (環境変数 `ENOMOTO_T_MIP_SEED` があればその値、なければ 0)。指定すればそれを優先する。
  - 範囲: 0 以上 2**64 未満の整数。負の値・2**64 以上は `ValueError`、整数でない値 (`bool` を含む) は `TypeError`。
    Python 側で確かめる。u64 に丸める形は、別の種が同じ種に化けて気づけないので採らなかった。
- 通り道 (Rust):
  - `MipSettings::seed: Option<u64>` から `MipParams::seed: u64` へ。`MipParams::default()` は `default_seed()`
    (= `tunable!("ENOMOTO_T_MIP_SEED")`、なければ 0)。
  - サブ MIP の `MipParams` (heuristics.rs 2 か所・heur_scip.rs 3 か所・mod.rs の CLQPRB) には親の `seed` を渡す。
- 種を直接読んでいた所を、パラメータの値を使う形にした:
  - solver.rs の `Solver::new` (木のヒューリスティクスの `rng`)。
  - solver.rs の `attach_prechain` の `salt`。
  - prechain.rs の `spawn` (`arm`・`start`・`attach` に種を渡す)。
  - mod.rs の TPRE の 2 か所 (`pt_spawn`・`tpre_job`)。
  - 前処理の中の VB01 のサブ MIP (`vbounds01_on_problem`): `solve_problem_inner` が立てる thread_local
    `PRESOLVE_SEED` から渡す。前処理の関数は `MipParams` を受け取らないため。
- 環境変数 `ENOMOTO_T_MIP_SEED` は計測の台本用に、引数が無いときの既定値として残した。
- 別の環境変数 `ENOMOTO_T_MIP_FJ_SEED`・`ENOMOTO_T_MIP_FEAS_FJ_EARLY_SEED` (FJ の種をさらにずらす試作の調整値) はそのまま。
- LP (整数変数のないモデル) では使わない。`solve_mip` は整数変数がなければ `MipParams` を作る前に `solve_lp` へ進み、`settings.seed` を見ない。
  LP の解法の中の乱数 (DSE の標本など) は固定の種で、この引数とは関係ない。
- 文書: README.md・README.ja.md の solve() の引数の表と「乱数の種 (`seed`)」の段落、`Model.solve` の docstring、Rust の `Model::solve` の説明。

## 2. 確かめ (事実)

- 種 0 (既定) で今と同じ動き (`check_s0.jsonl`、1 スレッド・120 秒)。変更前のコミット 45056c805 を別にビルドし、5 問で比べた。
  - 変更前 (引数なし)・変更後 (引数なし)・変更後 `seed=0` のノード数・目的値はすべて一致。
  - binkar10_1 3636・misc07 3766・mik-250-20-75-4 7361・neos-911970 7617・mas76 83717 ノード。
- 種 1 (`check_s1.jsonl`): 環境変数 `ENOMOTO_T_MIP_SEED=1` と引数 `seed=1` のノード数・目的値が 4 問で一致。
  2026-11-23 の nodespeed の計測 (環境変数で種 1) とも一致 (binkar10_1 3774・misc07 4004・mik-250 6333・mas76 121235)。
- tests/test_seed.py (pytest、全 64 件通過):
  - 1 スレッド・同じ種で 2 回解くとノード数・目的値が一致 (種 0・1・12345・2**64−1)。
  - `None` と 0 が同じ。
  - 種 0〜7 で目的値は同じでノード数が 2 通り以上 (40 列・5 行の 0-1 ナップサック。種 0 で 29 ノード、ほかに 43〜51 ノード)。
  - 範囲外は `ValueError`、型違いは `TypeError`。LP では受け取って使わない。
- cargo test・fuzz 7 本: 下の 3 節。

## 3. fuzz と cargo test

- `fuzz.sh` (analysis/nodespeed_20261123/fuzz.sh の写し) を、既定 (環境変数なし) と `ENOMOTO_T_MIP_SEED=7` で回した。
  - fuzz 7 本はどちらも誤答 0: tiny 0/400・0/400、vs HiGHS 0/300、objint 0/1000・0/1000、slack 0/1000・0/1000。
  - `cargo test --release --lib` はどちらも 423 通過・0 失敗・19 無視。
- 道具: `check.py` (1 スレッドで種を変えてノード数・目的値を出す)、`check_s0.jsonl`・`check_s1.jsonl`。
