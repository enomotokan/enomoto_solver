# cand1119 の誤答の確かめ (2026-11-17)

コミット df77337c3 で `pip install -e .` (maturin、リリース) で入れて回した (highspy 1.15.1、4 コア)。
`bash analysis/cand1119_20261117/fuzz.sh` で、cand1119 の環境変数
(`ENOMOTO_MIP_CLIQUE_CUTS=sigmoid ENOMOTO_T_MIP_CLIQUE_SIG_A=1.8 ENOMOTO_T_MIP_CLIQUE_SIG_B=0.41 ENOMOTO_MIP_SYM_TWINS=1 ENOMOTO_MIP_CUTSEL_PROBE_AFFORD=1`)
を付けて fuzz 8 本 (4 種 + 種を変えた 3 種 + 双子の併合の働く fuzz_dupcols) を順に回した。
cargo test は既定 (環境変数なし) で 1 回。ログは `fuzz_logs/` に出るがコミットしていない。

## 結果

| 命令 (cand1119 の環境変数付き) | 件数 | bad |
|---|---|---|
| `ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | 400 | 0 (strict 13) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300` | 300 | 0 |
| `python3 analysis/objint_20261112/fuzz_objint.py 1000 0` | 1000 | 0 (optimal 967・infeasible 33) |
| `python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0` | 1000 | 0 (mismatches 0。HiGHS: Infeasible 614・Optimal 268・Primal infeasible or unbounded 115・Unbounded 3) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000` | 400 | 0 (strict 14) |
| `python3 analysis/objint_20261112/fuzz_objint.py 1000 5000` | 1000 | 0 (optimal 971・infeasible 29) |
| `python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000` | 1000 | 0 (mismatches 0。HiGHS: Infeasible 655・Optimal 240・Primal infeasible or unbounded 101・Unbounded 4) |
| `python3 analysis/parint_20261028/fuzz_dupcols.py 1000` | 1000 | 0 |
| `CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib cargo test --release --lib` (既定) | 416 通過・0 失敗・19 無視 | — |

各 fuzz の要約行:

```
fuzz_tiny    bad 0 of 400 time 3.4 (strict 13: 値の一致だけの以前の判定)
fuzz_highs   bad 0 of 300 time 5.7
fuzz_objint  bad 0 of 1000 status {'optimal': 967, 'infeasible': 33} with nodes>0 307 new step 71 new step and nodes>0 3 c range with width 27 c and nodes>0 6 time 73.8
fuzz_slack   done 1000 (seeds 0..999): mismatches 0, HiGHS statuses {'Infeasible': 614, 'Optimal': 268, 'Primal infeasible or unbounded': 115, 'Unbounded': 3}
fuzz_tiny2   bad 0 of 400 time 3.4 (strict 14: 値の一致だけの以前の判定)
fuzz_objint2 bad 0 of 1000 status {'optimal': 971, 'infeasible': 29} with nodes>0 303 new step 75 new step and nodes>0 5 c range with width 27 c and nodes>0 2 time 71.8
fuzz_slack2  done 1000 (seeds 5000..5999): mismatches 0, HiGHS statuses {'Infeasible': 655, 'Unbounded': 4, 'Primal infeasible or unbounded': 101, 'Optimal': 240}
fuzz_dupcols bad 0 of 1000
cargo test   test result: ok. 416 passed; 0 failed; 19 ignored; 0 measured; 0 filtered out; finished in 2.80s
```

- **誤答 0、状態の食い違い 0** (8 本とも rc=0)。cargo test も 416 通過・0 失敗。
- fuzz_tiny の strict 13 / 14 は cand1118 (既定、analysis/cand1118_default_20261116/fuzz.md) と同じ数 (許容誤差の内の差)。
- fuzz_objint の状態の内訳は cand1118 と同じ。nodes>0 の数が 1 つずつ違う (308→307、302→303) のは木の進み方の差で、判定には影響していない。
- fuzz_tiny のログの `Presolve: the problem is infeasible` の後に `Result: optimal` となる箇所は 221 か所で、cand1118 と同じ (中で解く小さな問題の presolve の行)。
- 時間: fuzz_tiny 3.4 秒、fuzz_highs 5.7 秒、fuzz_objint 73.8 秒 (種 5000 側 71.8 秒)。cand1118 の回 (62.8 / 63.7 秒) より長いが、別の機械・時刻での 1 回ずつの比べで、確かめの目的ではない。
