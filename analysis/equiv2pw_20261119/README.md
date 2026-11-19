# 試作 `ENOMOTO_MIP_EQUIV2_PWORK` の確かめ (2026-11-19)

経緯は analysis/clqprbg_20261119/README.md。同値な組の巡 (`PROBE_CLIQUE=equiv`) の手間を、本体のプロービングの `pwork` と同じ重み
(2.5 x 行の走査 + 5 x 活動量の更新 + 68 x 境界の変更、およそナノ秒) で数え、`ENOMOTO_T_MIP_EQUIV2_PWORK_CAP` (5e8、0.5 秒相当) を越えたら止め、
`ENOMOTO_T_MIP_EQUIV2_PWORK_GIVEUP` (1e8) を越えて 1 組もなければやめる (`src/mip/mod.rs` の `equiv2_pwork`)。既定では使わない。

- 確かめ (`ENOMOTO_MIP_EQUIV2_PWORK=1`): cargo test 409 通過・0 失敗、fuzz 7 本 (release019 の fuzz.sh を環境変数つきで) すべて bad 0。
- 手元 (1 スレッド、種 1、前処理だけ): mzzv42z の CLQPRB の `g` の 1 周目の巡 2.98 → 0.74 秒 (402 → 63 組、どちらも使わない)、含意の形の巡
  0.77 → 0.59 秒 (1023 → 572 組、使わない)、ρ 3.424 で同じ。neos8 は巡の組の数・使う/使わないとも同じ (910・3239 組、使う)。
- 分担表 (`python plan.py`):
  - P (`shards_P.json`、2 分担・各約 1.4 時間): MIPLIB の 58 問 × 種 1〜3 × {def, epw}、60 秒・4 スレッド。
  - M (`shards_M.json`、2 分担・各約 1.4 時間): 11-07 の計測でプロービングが 2 秒以上で、final_20261118 の既定で 600 秒以内に解けた 16 問
    (air05・eil33-2・hypothyroid-k1・mushroom-best・mzzv11・mzzv42z・neos-4722843-widden・neos-860300・neos-957323・neos-960392・neos8・
    nursesched-sprint02・piperout-08・piperout-27・qap10・supportcase33) × 種 1〜3 × {def, epw}、600 秒・4 スレッド。
- 回し方 (試作を入れたコミット以降でビルド、`pip install -e .`): `cd analysis/equiv2pw_20261119 && python run.py P K` (K = 0・1)、`python run.py M K` (K = 0・1)。
- 集計: `python summ.py P epw > summ_P.txt`、`python summ.py M epw > summ_M.txt`。
