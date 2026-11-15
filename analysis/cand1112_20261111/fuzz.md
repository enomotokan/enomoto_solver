# 束 cand1112 の誤答の確かめ (2026-11-12)

固定のコミット 08d087d5b6a567e62d46056a6e394ea7384efcc8 を `/tmp/build` に取り出し、`maturin build --release` の車輪を入れて回した
(highspy 1.16.0)。束の環境変数 (`cfg.py` の CAND1112) を全部置いた:

```
ENOMOTO_MIP_ROOTSTOP=b ENOMOTO_MIP_RCSTRONG=irk ENOMOTO_MIP_LMIP_SWAP=1 ENOMOTO_MIP_SYMPRE=probe
ENOMOTO_MIP_PLSB=gxu ENOMOTO_MIP_GAP1=u ENOMOTO_MIP_GAP4=rocn
```

## 結果

| 命令 | 件数 | bad |
|---|---|---|
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | 400 | 0 (strict 17) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300` | 300 | 0 |
| `CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib cargo test --release --lib` | 400 通過・0 失敗・19 無視 | — |

- bad の行が出なかったので、環境変数を 1 つずつ外す切り分けは行っていない。
- fuzz_tiny の strict 17 (値の一致だけの以前の判定) は、同じコミット・同じ highspy で環境変数なし (既定) でも 17 で、
  食い違う seed (61, 81, 83, 139, 148, 159, 161, 195, 201, 210, 246, 260, 304, 305, 324, 362, 398) も同じ。
  全部「許容誤差の内の差」と判定されており (analysis/fuzz_tiny_judge_20261109.md)、束によるものではない。
  以前の記録の 13 との差は highspy の版などの違いと思われる (確かめていない)。
