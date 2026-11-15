# 束 cand1115 の誤答の確かめ (2026-11-15)

固定のコミット 41559edac を取り出し、`pip install -e .` (maturin、リリース) で入れて回した (highspy 1.15.1、4 コア)。
`bash analysis/cand1115_20261115/fuzz.sh` で、束の環境変数 (`cfg.py` の CAND1115、31 個 = CAND1114 + ADD1115 の 10 個) を全部置いて fuzz 6 種を順に回した。
cargo test は既定と CAND1115 の 2 回回した。生のログ (`fuzz_logs/`、1.4 MB) はコミットせず、要約だけ `fuzz_summary.txt` に置く。

## 結果

| 項目 (CAND1115 を全部付けて) | 回数 | 誤答数 | 備考 |
|---|---|---|---|
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | 400 | 0 | strict 13 (cand1114 と同じ)、2.6 秒 |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300` | 300 | 0 | 4.2 秒 |
| `python analysis/objint_20261112/fuzz_objint.py 1000 0` | 1000 | 0 | optimal 967・infeasible 33、60.4 秒 |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_symmetric.py 300` | 300 | 0 | 28.8 秒 |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_fixed_charge.py 200` | 200 | 0 | optimal 141・infeasible 59、2.0 秒 |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0` | 1000 | 0 | mismatches 0。HiGHS: Infeasible 614・Optimal 268・Primal infeasible or unbounded 115・Unbounded 3 |
| `cargo test --release --lib` (既定、RUSTFLAGS で pyo3 のリンクを回避) | — | — | 416 通過・0 失敗・19 無視 |
| 同じ、CAND1115 を付けて | — | — | 416 通過・0 失敗・19 無視 |

- **誤答 (bad・mismatch・wrong) 0**。このため ADD1115 の項目を 1 つずつ外す切り分けは行っていない。
- fuzz_tiny の strict 13 は cand1113・cand1114 と同じ数 (許容誤差の内の差、analysis/fuzz_tiny_judge_20261109.md)。
- 2 時間の上限には当たらず、全項目を 1 回で回し切った (全体で約 4 分)。
