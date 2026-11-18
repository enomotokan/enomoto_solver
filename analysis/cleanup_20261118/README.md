# 不採用の試作の片付け 第 1 弾 (2026-11-18)

依頼は `BRIEF.md`。ブランチ ccr-1beed81b-ex0uhq。1 件ごとに fetch+merge → 削除 → 確かめ → コミット → push。

## 確かめの方法

- `cargo build --release` の警告の行の数 (消す前 77)・`cargo test --release --lib` (消す前 422 通過・0 失敗・19 無視)。
  どちらも fuzz.sh と同じ設定 (`CARGO_TARGET_DIR=target/test-py`・python 3.13 を結ぶ `RUSTFLAGS`)。
- 既定の振る舞い: `ident.py` (1 スレッド・既定の種・60 秒、ENOMOTO_ の環境変数をすべて外す) で 12 問の (状態・目的値・ノード数) が
  消す前のビルド (`base.json`、ecb1ded51) と完全に一致すること。依頼書の例の pk1・neos5・10teams・gen-ip002 は 1 スレッドでは
  60 秒で終わらずノード数が時間で揺れるので、60 秒以内に解ける misc07・fiber・qnet1・dcmulti・blend2・exp-1-500-5-5・neos-860300・
  khb05250 に替えた。消す前のビルドで 2 回流して完全に一致することを確かめた。
- 4 件ごとと最後に、analysis/release019_20261117/fuzz.sh と同じ fuzz 7 本で bad 0。

## 1 件ごとの結果

| # | 試作 | コミット | 減った行 (src) | 警告 | cargo test | ident | 備考 |
|---|---|---|---|---|---|---|---|
| 1 | `PROBE_EQUIV2_RESTART` | — | — | — | — | — | **飛ばした**: 根拠「誤った最適値が出る」は記録と合わない。analysis/eqrst_20261113.md で誤答の原因 (係数の強化) は既定で直っていると確かめられ、`=1` は「正しさの手当てが要らなくなった制限を外す」束の候補として残されている (58 問の計測待ち)。分岐 1 行で、消すより束で決める方がよい |
| 2 | `QIU_MARGIN_DEPTH` | (このコミット) | 1 | 77 | 422/0/19 | 一致 | 根拠: analysis/qiu_margin_20261104.md (uccase12 で HiGHS の最適値より下の値・列 166 本の下限を破る)。調査用の `QIU_MARGIN_SITES` は依頼の外なので残した |
