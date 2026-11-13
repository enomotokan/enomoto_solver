# RCSTRONG3 の確かめの手順 (2026-11-13)

束 cand1114 の RCSTRONG を k・ik・irk で比べる (cfg.py)。記録は ../rcstrong3_20261113.md。

固定するコミット: このファイルを足したコミット (ccr-1beed81b-ex0uhq の先頭、blend2 の直し f1c14c30・dfb8186d を含む)。

```sh
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq
git checkout --detach <固定するコミット>
pip install maturin highspy==1.15.1 && pip install -e .
python analysis/rcstrong3_20261113/run.py {T|P} K      # 止まっても続きから再開できる (済んだ (問題, 形, 種) は飛ばす)
```

- T (狙いの 10 問 × 種 1〜4 × {k, ik, irk}、600 秒、120 回): 3 分担 (K = 0〜2、見積り各 1.4 時間)。
- P (58 問 × 種 1〜3 × {k, ik, irk}、60 秒、522 回): 6 分担 (K = 0〜5、見積り各 0.8 時間)。
- 同じ (問題, 種) の 3 形は同じ分担。終わったら res_{T|P}_sK.jsonl だけを名指しで add してコミット・push。
