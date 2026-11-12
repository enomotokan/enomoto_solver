# 束 cand1113 の誤答の確かめ (2026-11-12)

固定のコミット 81ca14d6 を取り出し、`pip install -e .` (maturin、リリース) で入れて回した (highspy 1.15.1、4 コア)。
束の環境変数 (`cfg.py` の CAND1113、26 個) を全部置いた。fuzz は 3 本を並べて走らせた (時間の数字はそのため長め)。

## 結果

| 命令 (CAND1113 を全部付けて) | 件数 | bad | 既定 (環境変数なし) |
|---|---|---|---|
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | 400 | 0 (strict 13) | 0 (strict 13) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300` | 300 | 0 | — |
| `python analysis/objint_20261112/fuzz_objint.py 1000 0` | 1000 | 0 (optimal 967・infeasible 33) | — |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0` | 1000 | **1 (seed 240、誤答ではなく状態が弱い)** | 0 |
| `CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib cargo test --release --lib` (既定) | 411 通過・0 失敗・19 無視 | — | |
| 同じ、CAND1113 を付けて | 411 通過・0 失敗・19 無視 | — | |

- fuzz_tubr は束に関係しないので回していない。fuzz_slack は今日の新しいほう (`analysis/wrongans_20261112/seed14/`) を使った。
- fuzz_tiny の strict 13 は束と既定で同じ数 (許容誤差の内の差、analysis/fuzz_tiny_judge_20261109.md)。

## fuzz_slack seed 240 (状態の食い違い 1 件)

HiGHS は Infeasible、束は `infeasible_or_unbounded`。値の誤り (実行不能な問題に最適解を返す等) ではなく、
「実行不能」と言い切れずに弱い状態を返したもの。fuzz_slack の判定は HiGHS が Infeasible のとき `infeasible` だけを正としているので食い違いに数えた。

- 環境変数を 1 つずつ外す・1 つだけ付ける切り分けで、**`ENOMOTO_MIP_HNFPRE=0` (prune1112) だけで起き、これを外すと消える**。
- 中身: 等式 7.5x1 − 4.5x2 − 7.5x4 − 7.5x5 − x8 = 7 (x は整数、x8 ∈ {0,1}) は x8 以外の部分が 1.5 の倍数なので 7 にも 8 にもならず、整数解がない。
  既定の HNF 前処理 (`bounds`) はこの合同式で実行不能を示すが、切ると緩和が非有界 (x2・x9・x11 で目的が下に伸びる) のまま進み、
  `infeasible_or_unbounded` で終わる。
- 再現: `slack240.py` (同じディレクトリ、行にも目的にも出ない列を除いた 12 列)。`python slack240.py` → infeasible、
  `ENOMOTO_MIP_HNFPRE=0 python slack240.py` → infeasible_or_unbounded。
