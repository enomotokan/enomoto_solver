# 試作 `ENOMOTO_MIP_GEN_LAZY` (余裕のある一般の不等式の行を LP から外し、破られたら戻す。2026-11-17、GENLAZY)

CLQ_LAZY (treeclose 9.80〜9.82 節、クリーク型の行だけ) を一般の行に広げた試作。既定では使わない。作りの説明は `src/mip/genlazy.rs` の頭。

## 切り替え

| 環境変数 | 意味 |
|---|---|
| `ENOMOTO_MIP_GEN_LAZY` | `1`・`all` で使う。`clq,one,vb,knap,int,mix,cont` のコンマ区切りで外す行の種類を選ぶ |
| `ENOMOTO_T_MIP_GEN_LAZY_THETA` | 余裕の判定のしきい値 θ (既定 0.5): 正規化した活動 a = (c·x − 最小活動)/(b − 最小活動) が θ 以下の行を外す (最小活動が無限なら a = 1/(1 + 余裕/‖c‖₂)) |
| `ENOMOTO_MIP_GEN_LAZY_MODE=score` | 点数の判定 (利用者の指示): カットの質 `cut_quality` と同じ点数 (効き目 + 0.1×目的との平行度 + 0.1×整数台、効き目は (c·x − b)/‖c‖ で満たされていれば負) が τ 未満の行を外す |
| `ENOMOTO_T_MIP_GEN_LAZY_TAU` | 点数の判定のしきい値 τ (既定 0) |
| `ENOMOTO_T_MIP_GEN_LAZY_AGE` | A > 0 なら、戻した行がノードの LP 解で A 回を超えて張り付かなければ外し直す (既定 0 = しない) |
| `ENOMOTO_MIP_GEN_LAZY_SB=1` | 強分岐の子の LP 解が破った外した行を、分枝の選びの後に戻してノードを解き直す |
| `ENOMOTO_MIP_GEN_LAZY_BACK=cut` | 戻すときカットの行として加える (CLQ_LAZY と同じ)。既定は元の行の境界を戻す |
| `ENOMOTO_T_MIP_GEN_LAZY_VIOL` | 破りの判定 c·x > b + tol·max(1,|b|) の tol (既定 1e-7) |
| `ENOMOTO_MIP_DIAG_GENLAZY` | 診断: 木に入るときの種類ごとの片側の行の数・スラックが基底の数・非零・外した数と、正規化した活動・点数の分布を `GENLAZY {...}` の 1 行で出し、終わりに `GEN_LAZY stats` を出す。`stop` なら外さない |

- 外すのは片側だけの不等式で、スラックが基底にある行だけ (両側の行・等式は外さない)。
- TREECLOSE の `ENOMOTO_MIP_DIAG_ROWTYPES` (`analysis/rowtypes_20261117/`) と役割を分けた: あちらは行の種類ごとの量・相対の余裕のおおまかな分布・
  大域の境界の下で冗長な行 (GREDUNDANT の見込み)、こちらは「木に入る時点で LP から外せるか」(正規化した活動・点数の細かい分布と外した行・戻した行) に絞る。

## 正しさ (`fuzz_*.txt`、`fuzz.sh`)

- `scripts/mip_fuzz_genlazy.py` (新規): 一般の不等式の行 (ナップサック・2 列・整数だけ・混ざり・連続だけ・クリーク型・1 列、範囲の無限な連続列も) の多い
  乱数の混合整数問題を HiGHS と照合。HiGHS の前処理が実行可能な問題を Infeasible と答えることがある (seed 157・10469・20232・34702・44996。
  前処理なしの HiGHS は ENOMOTO と一致) ので、食い違ったら前処理なしの HiGHS とも比べる。
- 前処理・カットなし (`ENOMOTO_MIP_NO_PRESOLVE=1 ENOMOTO_MIP_NO_CUTS=1`) で木に入らせた形と既定の形、1・4 スレッド、θ = 0.3・0.5・1、点数 τ = 0.3、
  外し直し (年齢 1・2)、強分岐の子の確かめ、カットで戻す形 (`fuzz_gen_*`・`fuzz_clq_*`・`fuzz_vs_*`・`fuzz_tiny_*` は最初の版 = カットで戻す、
  `fuzz_b_*` は境界で戻す、`fuzz_c_*` は点数と外し直し)。`mip_fuzz_clique.py`・`mip_fuzz_vs_highs.py`・`fuzz_tiny.py` も。**どれも食い違い 0**。
  前処理なしの形では 6〜7 割の問題で戻しが起き、戻した行の種類は全種類に渡る。
- cargo test 419 通過 (既定と `ENOMOTO_MIP_GEN_LAZY=1 ENOMOTO_T_MIP_GEN_LAZY_THETA=1` の両方)。

## 手元の確かめ (`local.py`、`res_local*.jsonl`、4 スレッド・種 1・300 秒、1 回ずつ)

`res_local_cutback.jsonl` はカットの行で戻す最初の版、`res_local.jsonl` は境界で戻す版。

| 問題 | def | θ 0.5 | θ 1.0 | θ 0.5 + 強分岐の子 |
|---|---|---|---|---|
| mzzv11 | 時間切れ (下界 −21806) | 197.9 秒 | 209.9 秒 | 176.2 秒 |
| neos-933966 | 188.9 秒 | 158.0 秒 | 時間切れ | 時間切れ |
| supportcase33 | 222.5 秒 | 118.5 秒 | 261.8 秒 | — |

- カットの行で戻す版では neos-933966 θ 0.5 の戻し 569 回のうち 270 回が「年齢で消された行の戻し直し」だったので、境界で戻す形を既定にした。
- 確かめの手間は小さい (mzzv11 θ 1.0 で外した 11.9 万非零、確かめ 1276 回で計 0.31 秒)。
- supportcase33 で外れるのは主に整数だけの行 (int 1834 行のうち θ 0.5 で 936 行) とクリーク型。

## 対の比べの台本

- `cfg.py` 設定、`plan.py NP NM` 分担表、`run.py SET K` 分担を回す、`summ.py SET` 集計 (各設定を def と比べ、問題ごとの外した非零の割合と比の並びも出す)。
