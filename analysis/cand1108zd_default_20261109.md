# cand1108zd を既定にする (2026-11-09)

- cand1108zd = cand1108 (analysis/mitt600_20261103.md の res6) + zerohalf なし (res7 の cand1108z) + 厳密な DSE 重みから始める `ENOMOTO_MIP_DSE_INIT=umr`
  (analysis/nodelp2_20261107.md の 10.2 節)。`analysis/halve_submip_20261030/run.py` の `CFGS["cand1108zd"]`。
- 根拠: Mittelmann の HiGHS に負ける 47 問 (600 秒・4 スレッド・種 1・2、res8、analysis/mitt600_20261103.md の最後の節): 既定 → cand1108zd で SGM 161.51 → 147.22
  (対の比 −8.3%、95% 区間 −17.3%〜+0.5%)、解けた 54 → 56 回、1 種でも解く問題 28 → 30。揃って速い mushroom-best (600・600 → 173・194 秒)・n2seq36q
  (514・287 → 76・78)・neos-860300、揃って遅い hypothyroid-k1 (32・43 → 86・83、zerohalf を止めたため)。res7 (186 問) では cand1108z が既定より解けた +6・SGM −1.7%。
- 残る課題: zerohalf が下界を支える問題 (enlight_hard・hypothyroid-k1・mzzv42z・comp07-2idx) で遅くなる (res7)。ns1208400・tbfp-network は cand1108 系で遅い (res6・res7。
  tbfp-network は umr で戻る向き)。

## 既定にした内容と戻し方 (戻す並びは run.py の `"precand1108zd"`)

| 設定 | 新しい既定 | 元の振る舞いに戻す |
|---|---|---|
| `ENOMOTO_SIMPLEX_SCALE` (`src/simplex/mip_lp.rs`) | `1` | `=0` |
| `ENOMOTO_T_SIMPLEX_SCALE_MIN_M`・`_MIN_RANGE` | 10000・1e4 | `=0`・`=0` |
| `ENOMOTO_SIMPLEX_HARRIS2` (`src/simplex/slope_intercept_dual.rs`) | `r` | `=0` |
| `ENOMOTO_MIP_MPSEP` (`src/mip/separation.rs`) | `oqmgs` | `=0` |
| `ENOMOTO_T_MIP_MPSEP_NV`・`_EXTRA` | 1・5 | `=2`・`=10` |
| `ENOMOTO_MIP_MUSHAGG` (`separation.rs`) | `cover` | `=0` |
| `ENOMOTO_MIP_PROBE_REPRESOLVE` (`src/mip/mod.rs`) | `1` | `=0` |
| `ENOMOTO_MIP_SUBFIX` (`src/mip/heur_subfix.rs`) | `rtcqf` | `=0` |
| `ENOMOTO_MIP_NODELP_PERTURB` (`src/mip/solver.rs`) | `phg` | `=0` |
| `ENOMOTO_MIP_BASIS_NB` (`mip_lp.rs`) | `t` | `=0` |
| `ENOMOTO_MIP_PROBE_BASIS_GUARD` (`solver.rs`・`mod.rs`) | `g` | `=0` |
| `ENOMOTO_MIP_HNFPRE` (`src/mip/hnf.rs`) | `bounds` | `=0` |
| `ENOMOTO_MIP_CUTLPCAP` (`separation.rs`) | `ac` | `=0` |
| `ENOMOTO_MIP_CMIRDELTA` (`src/mip/cmirdelta.rs`) | `n` | `=0` |
| `ENOMOTO_MIP_LPSCALE` (`mip_lp.rs`) | `h` | `=0` |
| `ENOMOTO_T_MIP_LPSCALE_MIN_RANGE` | 1e4 | `=0` |
| `ENOMOTO_MIP_NO_ZEROHALF` (`separation.rs`) | `1` (zerohalf を使わない) | `=0` (使う) |
| `ENOMOTO_MIP_DSE_INIT` (`slope_intercept_dual.rs`) | `umr` | `=0` |

- 切り替えは `env_def!` (付けていなければ既定の値、`0`・`off` で切る)。tunable は既定の値を変えた。
- これからの記録の `"def"` は cand1108zd を含む新しい既定。
