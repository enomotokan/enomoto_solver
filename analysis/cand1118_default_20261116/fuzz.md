# 0.1.8 に向けた誤答の確かめ (既定 = cand1118、2026-11-16)

コミット 2bf54bcff を取り出し、`pip install -e .` (maturin、リリース) で入れて回した (highspy 1.15.1、4 コア)。
`bash analysis/cand1118_default_20261116/fuzz.sh` で、環境変数なし (既定 = cand1118) で fuzz 4 種と、種を変えた 3 種を順に回した。
cargo test は既定で 1 回。ログは `fuzz_logs/` に出るがコミットしていない。

## 結果

| 命令 (既定) | 件数 | bad |
|---|---|---|
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | 400 | 0 (strict 13) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300` | 300 | 0 |
| `python analysis/objint_20261112/fuzz_objint.py 1000 0` | 1000 | 0 (optimal 967・infeasible 33) |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0` | 1000 | 0 (mismatches 0。HiGHS: Infeasible 614・Optimal 268・Primal infeasible or unbounded 115・Unbounded 3) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000` | 400 | 0 (strict 14) |
| `python analysis/objint_20261112/fuzz_objint.py 1000 5000` | 1000 | 0 (optimal 971・infeasible 29) |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000` | 1000 | 0 (mismatches 0。HiGHS: Infeasible 655・Optimal 240・Primal infeasible or unbounded 101・Unbounded 4) |
| `CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib cargo test --release --lib` (既定) | 416 通過・0 失敗・19 無視 | — |

各 fuzz の要約行:

```
fuzz_tiny    bad 0 of 400 time 2.9 (strict 13: 値の一致だけの以前の判定)
fuzz_highs   bad 0 of 300 time 4.7
fuzz_objint  bad 0 of 1000 status {'optimal': 967, 'infeasible': 33} with nodes>0 308 new step 71 new step and nodes>0 3 c range with width 27 c and nodes>0 6 time 62.8
fuzz_slack   done 1000 (seeds 0..999): mismatches 0, HiGHS statuses {'Infeasible': 614, 'Optimal': 268, 'Primal infeasible or unbounded': 115, 'Unbounded': 3}
fuzz_tiny2   bad 0 of 400 time 3.0 (strict 14: 値の一致だけの以前の判定)
fuzz_objint2 bad 0 of 1000 status {'optimal': 971, 'infeasible': 29} with nodes>0 302 new step 75 new step and nodes>0 5 c range with width 27 c and nodes>0 2 time 63.7
fuzz_slack2  done 1000 (seeds 5000..5999): mismatches 0, HiGHS statuses {'Infeasible': 655, 'Unbounded': 4, 'Primal infeasible or unbounded': 101, 'Optimal': 240}
cargo test   test result: ok. 416 passed; 0 failed; 19 ignored; 0 measured; 0 filtered out
```

- **誤答 0、状態の食い違い 0** (7 本とも rc=0)。
- fuzz_tiny の strict 13 は cand1114 と同じ数 (許容誤差の内の差、analysis/fuzz_tiny_judge_20261109.md)。種 1000 の側は strict 14。
- cargo test の通過数は cand1114 の 413 から 416 に増えた (テストの追加分)。
- fuzz_tiny のログには、木探索の表の後に `Presolve: the problem is infeasible` が出てから `Result: optimal` となる箇所が 221 か所ある。
  中で解く小さな問題 (sub-MIP など) の presolve の行が前置きなしで出ているものと見られ、判定 (bad) には影響していない。
- 時間: fuzz_tiny 2.9 秒、fuzz_highs 4.7 秒、fuzz_objint 62.8 秒 (種 5000 側 63.7 秒)。
