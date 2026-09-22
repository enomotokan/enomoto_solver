# FTRAN 結果密度の移動平均による密/疎切替(`docs/lu_comparison_enomoto_vs_highs.md` §2.7)

対象: `FtLu::should_use_dense_solve` の判定基準。基点 `main` = a9a326a(反復#8 の
`ft_refactor_trigger` 採用後)。4 コア、rustc 1.94.1、highspy(Python 3.11)、
NETLIB93 全問題。

指摘された問題: `should_use_dense_solve(rhs_nnz)` は **入力 nnz だけ** で密/疎を判定し、
HiGHS の `expected_density`(過去の FTRAN 結果密度の移動平均)に相当するものを持たない。
規模が上がるほど FTRAN 結果が予測より疎/密になり、切替が後手に回る。

## 0. 結論

(A/B 計測後に記入)

## 1. 現状の確認: 入力側の判定は実質的に一度も発火していない

`DENSE_RHS_FRACTION = 0.4` は **入力** rhs の非ゼロ率に対する閾値で、
`prof_phases::DENSE_RHS_BYPASSES` の既存コメントが記録しているとおり、実測で
全問題 0 回、つまり **主ループの FTRAN は常に Gilbert-Peierls 疎経路** を通っていた
(入基底列は元来疎で、BFRT の combined も 0.4·m を超えることは稀)。

一方 FTRAN の **結果** は疎とは限らない。反復#8 の分析
(`analysis/ft_refactor_trigger_20260922_040850.md` §2.4)が既に測っているとおり、
本ソルバの FTRAN 結果密度は HiGHS より高く(d2q06c 45% vs 13%、dfl001 49% vs 10%)、
eta 鎖の fill がそのまま出力密度に出る。入力の疎性は結果の疎性を保証しない。

## 2. 実装

- `sparse_lu::FtranDensity`: 呼び出し地点ごとに結果密度 `nnz/m` の移動平均を持つ。
  係数は HiGHS と同じ `kRunningAverageMultiplier = 0.05`
  (`DENSITY_AVERAGE_MULTIPLIER`)。
- `FtLu::should_use_dense_solve_tracked(rhs_nnz, &density)`:
  `should_use_dense_solve(rhs_nnz) || density.predicts_dense()`。
  **疎→密方向にしか動かさない**: 入力が 0.4·m を超える場合は GP の reach が
  それ以上になることが保証されるので、履歴で疎経路に引き戻す余地はない。
- 測定コスト 0: どの解法経路も最後に `out[col_perm[s]] = scratch[s]` の O(m)
  置換ループを通るので、そこで非ゼロを数える(分岐なしの `+= (v != 0.0) as usize`)。
  `solve_into` / `solve_into_capture` / `solve_sparse_into` /
  `solve_sparse_into_capture` が結果非ゼロ数を返す。
- **両分岐で記録する**ので、ゲートは固着しない。密経路に移った後に結果が疎に戻れば
  平均が下がり自動的に疎経路へ戻る(単体テスト
  `density_gate_flips_a_sparse_rhs_channel_dense_and_back` が固定)。
- 移動平均は `FtLu` ではなくソルバのループ側に置く。`FtLu` は再分解のたびに
  作り直されるため、基底が最も密になった瞬間に履歴を捨ててしまう
  (HiGHS も同じ理由で `HEkk` 側に持つ)。
- チャネルは 2 系統: 入基底列 FTRAN(`col_aq`)と BFRT combined(`bfrt`)。
  rhs の形が違い、密度も実測で 2 倍以上違う(§3)ので平均を混ぜない。
- 閾値 `EXPECTED_DENSE_FRACTION` は `ENOMOTO_EXPECTED_DENSITY_GATE` で上書き可能
  (`>= 1.0` でゲート無効 = 従来の入力のみ判定)。A/B はこの環境変数で、
  **同一バイナリ** に対して実施した(ビルド差によるノイズを排除)。

## 3. 計測1: ゲートはどこで何回発火するか(`ENOMOTO_PROF_PHASES_EXT`、gate=0.35)

新しい診断行 `density_gate_ftrans=<n> final_expected_density col_aq=<x> bfrt=<y>` は
「入力判定では疎経路に行ったはずの FTRAN のうち、結果密度ゲートだけを理由に
密経路へ回った回数」を数える。

| 問題 | 反復 | gate 発火 | 最終 expected(col_aq) | 最終 expected(bfrt) | wall(ms) |
|---|---:|---:|---:|---:|---:|
| dfl001 | 23,403 | 18,030 | 0.655 | 0.306 | 21,526 |
| pilot87 | 7,033 | 8,466 | 0.803 | 0.416 | 11,451 |
| fit2p | 5,870 | 10,357 | 0.999 | 0.487 | 2,818 |
| d2q06c | 6,383 | 4,504 | 0.663 | 0.319 | 2,111 |
| pilot | 3,151 | 3,186 | 0.874 | 0.420 | 1,713 |
| maros-r7 | 2,443 | 1,059 | 0.412 | 0.000 | 1,277 |
| greenbeb | 4,597 | 2,529 | 0.517 | 0.239 | 834 |
| 25fv47 | 3,221 | 2,598 | 0.744 | 0.371 | 389 |
| greenbea | 2,555 | 193 | 0.368 | 0.165 | 390 |
| degen3 | 1,930 | 1,009 | 0.591 | 0.318 | 341 |
| 80bau3b | 3,156 | 0 | 0.026 | 0.010 | 335 |

(発火回数が反復数を超える問題があるのは、1 反復に BFRT combined と入基底列の
2 チャネル分の FTRAN があり、両方が発火しうるため。)

- **59/93 問題**でゲートが 1 回以上発火。それらの問題は全 93 問題の壁時計の
  **98.6%**(44.9s / 45.6s)を占める。
- 入基底列チャネルの最終 expected 密度は重い問題ほど高い(dfl001 0.655、
  pilot87 0.803、fit2p 0.999、pilot 0.874)。**入力は疎でも結果はほぼ密**という、
  §2.7 が指摘したとおりの状況が実在する。
- 例外は 80bau3b(0.026)。ここは結果が本当に疎で、ゲートは一度も発火しない。
  これは反復#8 で CLOCK トリガが eta 鎖を短くした効果でもある。

## 4. 計測2: 閾値の A/B(93 問題、同一バイナリ、環境変数で切替)

(記入予定)

## 5. 採否

(記入予定)
