# 群 2 kcs: `ENOMOTO_MIP_KEEP_COSTED_SINGLETON=1` の計測 (2026-11-18)

試作の説明と kakapo の結果は tree2/README.md 10 節。設定は `cfg.py` (def・kcs)。台本は domrev/ の写し。

- 働く問題の見当: `count.py` → `count.txt` (元の MPS で、費用のついた連続列が等式 1 本だけに出る数)。0 より大きいのは P 10 問
  (neos-860300・physiciansched6-2・swath1・markshare_4_0・misc07・qnet1・rout・binkar10_1・blp-ar98・timtab1) と M 13 問 (CMS750_4・
  istanbul-no-cutoff・mcsched・neos-3216931-puriri・neos-3627168-kasai・neos-5107597-kakapo・sct2・snp-02-004-104・timtab1・tr12-30・uccase12・
  unitcal_7、ほかに niemur を足した)。1 本だけの問題は目的の値の列 (z = c x の形) が多い。
- P = MIPLIB の 58 問 × 種 1〜3 × {def, kcs}、60 秒、4 分担 (各 0.74 時間): `cd analysis/vsscip_slow_20261118/kcs && python run.py P K` (K = 0〜3)
- M = 上の M の 13 問 × 種 1・2 × {def, kcs}、600 秒、5 分担 (各 1.7〜2.0 時間): `python run.py M K` (K = 0〜4)
- 集計: `python summ.py P kcs`・`python summ.py M kcs`。働かない問題 (count 0) は既定と同じ計算なので揺れの目安として分けて見る。
