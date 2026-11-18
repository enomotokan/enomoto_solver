# 群 2 kcs: `ENOMOTO_MIP_KEEP_COSTED_SINGLETON=1` の計測 (2026-11-18)

試作の説明と kakapo の結果は tree2/README.md 10 節。設定は `cfg.py` (def・kcs)。台本は domrev/ の写し。

- 働く問題の見当: `count.py` → `count.txt` (元の MPS で、費用のついた連続列が等式 1 本だけに出る数)。0 より大きいのは P 10 問
  (neos-860300・physiciansched6-2・swath1・markshare_4_0・misc07・qnet1・rout・binkar10_1・blp-ar98・timtab1) と M 13 問 (CMS750_4・
  istanbul-no-cutoff・mcsched・neos-3216931-puriri・neos-3627168-kasai・neos-5107597-kakapo・sct2・snp-02-004-104・timtab1・tr12-30・uccase12・
  unitcal_7、ほかに niemur を足した)。1 本だけの問題は目的の値の列 (z = c x の形) が多い。
- P = MIPLIB の 58 問 × 種 1〜3 × {def, kcs}、60 秒、4 分担 (各 0.74 時間): `cd analysis/vsscip_slow_20261118/kcs && python run.py P K` (K = 0〜3)
- M = 上の M の 13 問 × 種 1・2 × {def, kcs}、600 秒、5 分担 (各 1.7〜2.0 時間): `python run.py M K` (K = 0〜4)
- 集計: `python summ.py P kcs`・`python summ.py M kcs`。働かない問題 (count 0) は既定と同じ計算なので揺れの目安として分けて見る。

## P の結果と kcsc (2026-11-18)

- P (分担 0・2・3、162d00ca3、`summ_split_P.txt`): 働く 10 問 SGM 1.140 (markshare_4_0 2.13 倍・misc07 1.49・timtab1 1.23)、働かない 48 問 1.000。
- markshare_4_0 (1 スレッド、3 種): 既定 6.3〜9.2 秒・26.5 万ノード、kcs 21.5〜22.1 秒・43.2 万ノード。残したスラック 4 本の等式のほかの列は 0-1 列
  (費用が 0-1 列に移る形)。kakapo は等式のほかの列が連続 (開始時刻)。
- 試作の値 `c` (kcsc): 等式のほかの列がすべて連続で、上下限の片方が無限の列だけ残す。1 スレッドで P の働く 10 問は前処理もノードも既定と同じに戻り
  (timtab1 は有界のスラックを外して 44.6 → 43.1 秒)、kakapo の根は 538.9 のまま。M の働く 13 問 + niemur で前処理が既定と変わるのは
  kakapo (列 3045 → 3057) と snp-02-004-104 (140341 → 141275) だけ。
- 確かめ: cargo test --lib 409 通過、fuzz (HiGHS 400・objint 600・slack 600) 誤答 0。
- 短い計測 C (`shards_C.json`): kakapo・snp-02-004-104 × 種 1〜3 × {def, kcsc}、600 秒、2 分担 (各 1 時間): `python run.py C K` (K = 0・1)。

## C の結果 (68ad4915c、600 秒・4 スレッド、res_C_s0・s1)

| 問題 | 種 | def 暫定解 / 下界 | kcsc 暫定解 / 下界 |
|---|---|---|---|
| kakapo | 1 | 5688 / 176 | 7821 / 767 |
| kakapo | 2 | 5994 / 298 | 5454 / 826 |
| kakapo | 3 | 11160 / 109 | 6615 / 775 |
| snp-02-004-104 | 1〜3 | 種 3 だけ 546 秒で最適、ほかは時間切れ | 3 種とも時間切れ (下界は種 1・2 で def よりわずかに高い) |

- kakapo は 3 種とも根の下界が 1 前後 → 418〜624、最後の下界も 3〜7 倍 (SCIP は 600 秒で 1172.7)。暫定解は種で混ざる (SCIP 3672)。
- snp-02-004-104 は 3 回では揺れの範囲 (def が 1 回だけ閉じた)。
- P・M のほかの問題では kcsc は前処理が既定と同じ (1 スレッドで確かめた) ので、SGM・解けた数は変わらない。
