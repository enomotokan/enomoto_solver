# 束 cand1114 の誤答の確かめ (2026-11-13)

固定のコミット 54a057b8 を取り出し、`pip install -e .` (maturin、リリース) で入れて回した (highspy 1.15.1、4 コア)。
`bash analysis/cand1114_20261113/fuzz.sh` で、束の環境変数 (`cfg.py` の CAND1114、21 個) を全部置いて fuzz 4 種を順に回した。
cargo test は既定と CAND1114 の 2 回回した。ログは `fuzz_logs/` に出るがコミットしていない。

## 結果

| 命令 (CAND1114 を全部付けて) | 件数 | bad |
|---|---|---|
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | 400 | 0 (strict 13) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300` | 300 | 0 |
| `python analysis/objint_20261112/fuzz_objint.py 1000 0` | 1000 | 0 (optimal 967・infeasible 33) |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0` | 1000 | 0 (mismatches 0。HiGHS: Infeasible 614・Optimal 268・Primal infeasible or unbounded 115・Unbounded 3) |
| `CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib cargo test --release --lib` (既定) | 413 通過・0 失敗・19 無視 | — |
| 同じ、CAND1114 を付けて | 413 通過・0 失敗・19 無視 | — |

- **誤答 0、状態の食い違い 0**。cand1113 で出た fuzz_slack seed 240 (`ENOMOTO_MIP_HNFPRE=0` による `infeasible_or_unbounded`) は、
  束から HNFPRE=0 を外したので出ていない。
- fuzz_tiny の strict 13 は cand1113 と同じ数 (許容誤差の内の差、analysis/fuzz_tiny_judge_20261109.md)。
- 時間: fuzz_tiny 4.4 秒、fuzz_highs 6.6 秒、fuzz_objint 103.0 秒。
