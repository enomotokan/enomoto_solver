# 試作 LAZYFRAC (外せる行の非零の割合 f のシグモイドで、GENLAZY・CLQ_LAZY の外す量を決める。2026-11-17〜18)

GENLAZY (`analysis/genlazy_20261117/`) と CLQ_LAZY (treeclose 9.80〜9.86 節、`analysis/clqtheta_20261117/`) は P では中立、M では mzzv11・neos-933966 で速く
supportcase33・timtab1・qiu・mc11 などで遅かった。親の振り返りでは、遅れた問題は外す非零の割合 f (外す候補の非零 / 元の行の非零) が小さく、
f の大きい問題は中立〜速い。利用者の方針 (パラメータは問題の特徴の連続 (シグモイド) 関数) に沿って、外す量を f のシグモイドで決める門を足した。既定は不変。

## 切り替え (どれも既定では使わない)

| 環境変数 | 意味 |
|---|---|
| `ENOMOTO_T_MIP_GEN_LAZY_GATE_F0` | f0 > 0 なら GENLAZY の門を使う。木に入るとき (主) の候補 (θ・τ の判定で外す行) の非零の割合 f で share = σ(k (f − f0)) を求め、余裕の大きい順 (点数の判定なら点数の低い順、余裕の判定なら正規化した活動の小さい順) に floor(share × 候補) 行だけ外す |
| `ENOMOTO_T_MIP_GEN_LAZY_GATE_K` | GENLAZY の門の傾き k (既定 20) |
| `ENOMOTO_T_MIP_CLQ_LAZY_GATE_F0`・`_K` | CLQ_LAZY の同じ門 (前からあった変数。順は文字の和の小さい順) |
| `ENOMOTO_MIP_CLQ_LAZY_GATE_FALL=1` | CLQ_LAZY の f を、θ の候補ではなく余裕のある (1 − 文字の和 > 1e-6) クリーク型の行全部で数える (θ を小さくすると f が問題を分けなくなるため) |
| `ENOMOTO_MIP_LAZY_GATE_ONE=1` | 対照: share >= 0.5 (= f >= f0) のとき余裕の最も大きい 1 行だけ外し、そうでなければ外さない (GENLAZY・CLQ_LAZY 共通。mzzv11 の「1 行外すだけで速い」偶然の分を測る) |
| `ENOMOTO_MIP_DIAG_GENLAZY=exit` | 診断だけ: GENLAZY の行 (と CLQ_LAZY も使うなら `CLQLAZY_F` の行) を出して木に入らずに終える (`count_f.py` が使う)。`stop`・`exit` では CLQ_LAZY も外さない |

- 門は主 (写しなし) だけで決め、従は主の一覧を使う (今までと同じ)。木の中の再スタートで木に入り直すたびに数え直す (timtab1 では入り直すごとに f が 0.18 → 0.33 と増えた)。
- 診断 (`ENOMOTO_MIP_DIAG_GENLAZY` が何かの値) のとき、門の行 (`GEN_LAZY gate: ...`・`MIP: CLQ_LAZY gate: ...`)、`CLQLAZY_F {...}` (CLQ の f・f_all)、
  終わりの `CLQ_LAZY stats` (詳しい表示でなくても) を出す。
- share は連続なので、f が小さい問題でも候補が多いと数行は外れる (supportcase33 で share 0.003 × 1712 行 = 4 行)。

## 手元の当たり (`local_L.out`・`res_L_s0.jsonl`、4 スレッド・種 4 (門の選びに使っていない種)・1 回ずつ)

設定は `cfg.py`: gs = GENLAZY 点数 τ 0 + 門 (f0 0.3、k 40)、cs = CLQ_LAZY θ 0.1 + 門 (f は余裕のある行全部、f0 0.12、k 150)、g1 = gs と同じ門で 1 行だけ。

| 問題 (秒) | GEN の f / CLQ の f_all | def | gs | cs | g1 |
|---|---|---|---|---|---|
| noswot (60) | 0.41 / — | 時間切れ | 5.2 (65 行) | 10.5 (0 行) | 8.4 (1 行) |
| mzzv11 (300) | 0.59 / 0.70 | 時間切れ (下界 −22056) | **275.2** (4243 行) | **272.0** (3171 行) | 時間切れ (下界 −22056、1 行) |
| neos-933966 (300) | 0.26 / 0.15 | **145.4** | 時間切れ (share 0.15、223 行、戻し 0) | 時間切れ (409 行) | 153.9 (0 行) |
| supportcase33 (300) | 0.15 / 0.10 | 時間切れ (−395) | 時間切れ (−385、4 行) | 時間切れ (−390、0 行) | — |
| timtab1 (300) | 0.18〜0.33 | 20.9 | 25.1 (入り直し後に 90 行) | — | — |

- **mzzv11 は種 4 では 1 行だけ (g1) では効かず、たくさん外す gs・cs で効いた**。genlazy の種 1 の「1 行だけで効く」は種によるらしく、種 4 では外す量の効き目に見える (1 回ずつ)。
- **neos-933966 は種 4 では外すと遅い** (def 145 秒が gs・cs で時間切れ)。genlazy・clqtheta の種 1・2 では速かった問題で、種で向きが変わる (def 自身も種で 2 倍動く)。
- noswot は cs で何も外さないのに def の 60 秒切れが 10 秒になる (4 スレッドの揺れ)。
- 門は supportcase33・timtab1 (の最初の入り) で狙いどおりほぼ何も外さない。

## Mittelmann 186 問の f (`count_f.py mitt`、`count_f_0of2.jsonl`・`count_f_1of2.jsonl`、2 スレッド × 2 本並べて・150 秒、木に入るときに数えて終える)

156 問が木に入った (残りは根で解けたか 150 秒内に木に届かず)。GEN = GENLAZY 点数 τ 0 の候補の非零の割合、CLQ = 余裕のあるクリーク型の行全部の非零の割合 (f_all)。
GEN ≥ 0.3 か CLQ ≥ 0.12 の問題は 66 問 (多くは def が 600 秒で解けない)。主なもの:

- GEN が大きい: supportcase42 0.98、seymour1 0.89、ns1830653 0.86、app1-1 0.85、glass4 0.83、seymour 0.76、graphdraw-domain 0.75、rocII-5-11 0.74、bnatt400/500 0.74、
  app1-2 0.72、net12 0.70、glass-sc 0.67、istanbul-no-cutoff 0.64、neos-1456979 0.61、rocI-4-11 0.59、mzzv11 0.59、puriri 0.53、mzzv42z 0.36、kasai 0.36、
  neos-4722843-widden 0.43、CMS750_4 0.42、neos-787933 0.40、tr12-30 0.33、unitcal_7 0.32、fiball 0.31。
- CLQ が大きい: sorrell3 0.99 (GEN は 0.05: 余裕は小さいが 1e-6 は超える行ばかり)、mzzv11 0.70、mzzv42z 0.49、nexp-150-20-8-5 0.42、gmu-35-40 0.35、mcsched 0.26、
  puriri 0.24、reblock115 0.21、rail01 0.20、comp21-2idx 0.17、neos-933966 0.15。

M に足した 12 問 (`mprobs_extra.json`): f が門を越え、def (vshighs_20261117 の 600 秒) で解けて比が測れる問題 — ns1830653・graphdraw-domain・istanbul-no-cutoff・
rocI-4-11・mzzv42z・neos-4722843-widden・CMS750_4・tr12-30・unitcal_7・mcsched (CLQ 0.26)・neos-787933・fiball。genlazy の M の 7 問と合わせて 19 問。

## 正しさ

- cargo test 419 通過 (既定、GENLAZY 点数 τ 0 + 門 f0 0.1、CLQ_LAZY θ 0.1 + FALL + 門 f0 0.05 + 1 行だけ)。
- fuzz (`fuzz.sh`、`fuzz_*.txt`。門がよく働くよう f0 を小さく: GEN f0 0.1・k 20、CLQ f0 0.05・k 20): GENLAZY 前処理・カットなし 1・4 スレッド、既定 4 スレッド、
  1 行だけ 4 スレッド、一般 (`mip_fuzz_vs_highs.py`) 1 スレッド、CLQ_LAZY (`mip_fuzz_clique.py`) 前処理なし 4 スレッド・既定 4 スレッド・1 行だけ 1 スレッド、各 300 問。
  **誤答 0**。ただし前処理なし 1 スレッドの 2 組で「最適値は合っているが状態が not_solved」が 1・3 問出た。**既定 (環境変数なし) でも同じ種 (vs_highs 136・256・276) で出る**
  (`fuzz_vs_nopre_t1_default_only.txt`) もので、他の分担のコミット ec54d0cb0 (際どい LP の実行不能の確かめ。強分岐の子を「不明」にしてノードを LP なしで分枝) 由来:
  `ENOMOTO_MIP_NO_LPINF_CHECK=1` で消え、その形で回し直した 2 組 (`*_nolpinf.txt`) は食い違い 0。
- 門の行の数 (`gate lines`) と外した行: 前処理なしの GENLAZY では 300 問中 220 問で外し、戻しも起きる (232 本)。

## 計測の依頼 (P 6 分担・M 10 分担)

設定は def・gs・cs・g1 (`cfg.py`)。同じ (問題, 種) の 4 設定は同じ分担で続けて解く。**この README のコミット以降でビルドして回す** (`maturin develop --release`)。ソルバーは同時に 1 つ。

- P: 58 問 × 種 4〜6 × 4 設定・60 秒 (696 回)、6 分担 (各約 0.95 時間): `python run.py P K` (K = 0〜5)。
- M: 19 問 × 種 3・4 × 4 設定・600 秒 (152 回)、10 分担 (各約 1.4〜1.5 時間、def の秒 × 1.3 で見積り): `python run.py M K` (K = 0〜9)。
- 集計: `python summ.py P > summ_P.txt`、`python summ.py M > summ_M.txt` (各設定を def と比べ、問題ごとの f・外した行・比、M は (問題, 種) ごとの秒と時間切れの下界)。
- 分担表を作り直すなら `python plan.py 6 10`。
