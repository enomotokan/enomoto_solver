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

## 結果 (4 分担そろい、ビルド 755711147、4 スレッド)

比は「epw / def」(< 1 なら試作が速い)。SGM は shift 1 秒、区間は問題単位の 95% ブートストラップ。

| 組 | 行 | SGM の比 | 95% 区間 | 解けた数 def → epw | 全種で 1.25 倍超 | 食い違い |
|---|---|---|---|---|---|---|
| P (58 問 × 種 1〜3、60 秒) | 348 | **−2.0%** | −4.6% 〜 +0.2% | 138 → 139 | 遅い側なし (neos-911970 が全種で速い 0.71) | 0 |
| M (16 問 × 種 1〜3、600 秒) | 96 | **−1.0%** | −4.1% 〜 +2.0% | 48 → 48 | なし | 0 |

- 採否の基準 (SGM の 95% 上限 ≲ +5%・解けた数を減らさない・全種で 1.25 倍超の遅れなし・誤答 0) をどちらも満たす。P の 1 つ多く解けた組は
  mzzv42z 種 1 (epw 59.8 秒、def は 60 秒で時間切れ)。
- 1 問の比が大きいもの: P の qiu 1.18 (1.06・1.19・1.29、全種で遅いが 1.25 倍未満)、mc11 0.68、neos-911970 0.71 (全種で速い)。M の qap10 0.85。
- M の分担 1 で epw が遅かった mzzv42z 種 3 (1.37)・supportcase33 種 2 (1.52) などは、前処理の後の問題が def と同じ (md5 一致、前処理は
  15.2 → 10.5 秒など短い) で、4 スレッドの道の揺れ (分担 0 では supportcase33 種 3 が逆に 0.75)。前処理の後の問題が変わるのは調べた 4 問中
  neos-4722843-widden だけ。
- 勧め: 既定にする (利用者の判断)。手間で予算を決める方針 (`ENOMOTO_MIP_PRESOLVE_WORK`) の数え漏れの直しで、効き目は前処理の秒の短縮
  (mzzv42z で約 4〜5 秒) と、時間の安全網に頼っていた巡の打ち切りが手間で決まること (道がスレッドの数・機械の速さによらなくなる)。
