# cand1111 の誤答検査 (2026-11-11)

束 cand1111 の環境変数 (cfg.py の CAND1111、`checks/env.sh`) を全部立てて、既存の誤答検査を回した。
依頼のコミット 08d087d5 は origin に見つからなかったので、ブランチの先頭 794f7580 (cand1111 M38 完了) の状態で回した。
Python の束縛は `maturin build --release` で作り直して入れた。highspy 1.15.1。

| 検査 | 結果 |
|---|---|
| `benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | bad 0 of 400 (strict 13。既定でも 13 で、許容誤差の内の差。analysis/cutmix_20261111.md) |
| `ENOMOTO_MIP_REL_GAP=1e-9 scripts/mip_fuzz_vs_highs.py 300 0` | bad 0 of 300 (状態の食い違い・目的値の食い違い なし) |
| `cargo test --release` (RUSTFLAGS="-L /usr/local/lib -l python3.13") | 400 passed, 0 failed, 19 ignored |

- 誤答が出なかったので、束の項目を外す切り分けはしていない。
- `cargo test --release` は RUSTFLAGS なしではリンクで失敗する (pyo3 の Python の記号が未解決、束とは関係ない)。
- 記録: `checks/fuzz_tiny.log.gz`、`checks/fuzz_vs_highs.log.gz`、`checks/cargo_test.txt`。
