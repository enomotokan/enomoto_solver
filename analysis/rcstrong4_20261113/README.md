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
