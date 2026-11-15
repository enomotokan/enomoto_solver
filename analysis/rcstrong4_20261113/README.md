# RCSTRONG4 の確かめの手順 (2026-11-13)

試作 `ENOMOTO_MIP_RCSTRONG=gk` (src/mip/rcstrong.rs の `g`: `i` の根の切り捨てを保留し、根の再スタートを決めたときだけ当てる) を、
束 cand1114 (RCSTRONG=k) の上で k・ik と比べる (cfg.py)。記録は ../rcstrong4_20261113.md。

固定するコミット: このファイルを足したコミット (`g` の実装を含む)。

```sh
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq
git checkout --detach <固定するコミット>
pip install maturin highspy==1.15.1 && pip install -e .
python analysis/rcstrong4_20261113/run.py {T|P} K      # 止まっても続きから再開できる
```

- T (狙いの 10 問 × 種 1〜4 × {k, ik, gk}、600 秒、120 回): 3 分担 (K = 0〜2、見積り各 1.4 時間)。問題は ../rcstrong3_20261113 と同じ。
- P (58 問 × 種 1・2 × {k, gk}、60 秒、232 回): 3 分担 (K = 0〜2、見積り各 0.7 時間)。
- 終わったら res_{T|P}_sK.jsonl だけを名指しで add してコミット・push。集計は `python summ.py > summ.txt`。

## 2 回目 (直した gk、2026-11-13)

1 回目の P・T で gk が pk1 を 2 倍遅くした (保留は当てていない)。暗黙の整数の連続列が `k` の行に入るのが元で、`g` のときは `k` の行に整数列だけを
入れる (`ENOMOTO_MIP_RCSTRONG_GK_INT`、既定で入る) 形に直した。固定するコミットは、この節を足したコミット。

```sh
python analysis/rcstrong4_20261113/run.py {T2|P2} K     # K = 0〜2
```

- T2 (狙いの 10 問 × 種 1〜4 × {k, gk}、600 秒、80 回): 3 分担 (各 0.95 時間)。P2 (58 問 × 種 1・2 × {k, gk}、60 秒、232 回): 3 分担 (各 0.7 時間)。
- 集計は `python summ.py 2 > summ2.txt`。

## 3 回目 (P3: 2 回目の P に種 3・4 を足す)

固定するコミットは 2 回目と同じ d857365d (ソースは同じ)。台本だけ新しくする:

```sh
git checkout --detach d857365d
git checkout origin/ccr-1beed81b-ex0uhq -- analysis/rcstrong4_20261113
pip install maturin highspy==1.15.1 && pip install -e .
python analysis/rcstrong4_20261113/run.py P3 K      # K = 0〜2 (58 問 × 種 3・4 × {k, gk}、60 秒、232 回、各 0.7 時間)
```

集計は `python summ.py 2 > summ2.txt` (P は P2 と P3 を合わせて種 1〜4)。
