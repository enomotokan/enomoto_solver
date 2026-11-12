# cand1113 の確かめ (2026-11-12)

束 cand1113 = cfg.py の `CAND1113` (今日までに合格した 11 項目 + cand1112 の 7 項目 + OBJINT=hfc + CLQPRB=1d)。WRONGANS の seed14 の直しは既定に入っているので両側に入る。
入れないもの: FIXMORE (保留)・TREEEFF の D (保留)・CLQACT・TUBR・クリーク表の持ち方 (次の束で判断)・CUTRND・CLIQFB の各案・CLQEXT_KNAP (見送り)。
既知の注意: RCSTRONG は rmatr100-p10、GAP1=u は pk1 を遅くする (WINWHY、120 秒・種 2 つ)。physiciansched6-2 は揺れが大きいので種 5〜12 も回す。

## 手順 (分担 1 つ = セッション 1 つ)

固定するコミット: このファイルを足したコミット (分担の依頼に書く)。

```sh
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq
git checkout --detach <固定するコミット>
pip install maturin highspy==1.15.1 && pip install -e .      # リリースのビルド (20〜25 分)
python analysis/cand1113_20261112/run.py {M|P} K              # 途中から再開できる。ソルバーは同時に 1 つ (4 スレッド)
```

- M (Mittelmann 186 問 × 種 1・2 × {def, cand1113} 600 秒 + 種 1 の HiGHS): 28 分担 (見積り各 約 4 時間)。同じ問題の回は同じ分担。
- P (58 問 × 種 1〜4 × {def, cand1113} 60 秒 + physiciansched6-2 の種 5〜12): 6 分担 (見積り各 約 1 時間)。
- 終わったら res_{M|P}_sK.jsonl だけを名指しで add してコミットし push する (`git add -A` は使わない)。
