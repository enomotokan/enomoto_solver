# 試作の第 1 段: P でのふるい分け (2026-11-24、台本と分担表)

負け問題の調査 (analysis/losses_20261123 の後の各調べ) で作った試作 11 個を、P (MIPLIB の 58 問、analysis/cbcimpl_20261029/probs.py) × 種 1〜3・60 秒で
既定と比べ、P で悪くしないものを選ぶ。計測は親が割り当てる。

## 1. 設定 (cfg.py)

環境変数の名前と値は、各記録とコード (`src/`) の説明で確かめた。どれも既定オフ。

| 番号 | 名前 | 環境変数 | スレッド | 記録 |
|---|---|---|---|---|
| 1 | h1t | ENOMOTO_MIP_HEUR1T=fl | 1 | heur1t_20261123 |
| 2 | fps | ENOMOTO_MIP_FP_STAGE3=1・POOR_NOINC=1・P2RIT=a・FJSYNC_LABEL=1 | 1 | presym_20261123 (FJSYNC_LABEL も入れた: 1 スレッドの同期の FJ の解に名札を付ける。組で CMS750_4 を確かめた形) |
| 3 | enum | ENOMOTO_MIP_ENUM_PRESOLVE=all・ENUM_WHERE=loop・PRR_TIGHT=1 | 1・4 | cutgap_20261123 |
| 4 | ncl | ENOMOTO_MIP_NODE_CLEANUP=qa | 1・4 | nodespeed_20261123 |
| 5 | pool1 | ENOMOTO_MIP_POOL_SEL=1 | 1・4 | nodespeed_20261123 |
| 6 | poole | ENOMOTO_MIP_POOL_SEL=e | 1・4 | nodespeed_20261123 |
| 7 | hso | ENOMOTO_MIP_HO4_ACFIX=d・SB_NO_DEGEN_SKIP=1・SB_BUDGET_HIGHS=1 | 1・4 | highsonly_20261123 |
| 8 | lux | ENOMOTO_MIP_LU_EXTEND=1 | 1・4 | lpiter_20261123 |
| 9 | bfrt | ENOMOTO_MIP_NODELP_BFRT_GROUP=1 | 1・4 | lpiter_20261123 |
| 10 | ppi | ENOMOTO_MIP_PROOFPROP=i | 4 | proofprop_20261123 |
| 11 | sbpb | ENOMOTO_MIP_SB_PROBEBD=1 | 1・4 | treegap_20261123 |

- 既定 def_t1 (1 スレッド)・def_t4 (4 スレッド) は 1 回だけ回し、全試作と共有する。ただし同じ (問題, 種) の 21 設定 (既定 2・試作 19) は**同じ分担で続けて**
  解くので、試作と既定は同じ機械・同じ時期の組になる (共有でも組でもある)。
- 衝突の重み 0.01 は今回は入れない (親の指示)。名前が `_t1` で終われば 1 スレッド、`_t4` なら 4 スレッド (run.py の TH)。

## 2. 問題と分担 (plan.py → shards.json)

- 58 問 × 種 1〜3 × 21 設定 = **3654 回**、60 秒、**8 分担**、見積り各 4.17〜4.22 時間 (2 時間弱 × 2〜3 回)、計 33.6 時間。
  - 見積り: analysis/final_20261118 の P の def・def_t1 の同じ (問題, 種) の時間 (解けなければ 60 秒)、試作も同じとみて、1 回ごとに待ち 8 秒を足した。
  - 同じ (問題, 種) の設定の順は組ごとに回して入れ替える。作り直しは `python plan.py 8`。
- 回し方: `python analysis/protoscreen_20261124/run.py K` (K = 0〜7) → res_sK.jsonl。止めても同じコマンドで続きから回る (済んだ (問題, 設定, 種) は飛ばす)。
- 集計: `python analysis/protoscreen_20261124/summ.py > summ.txt`。

## 3. ビルドと、既定が変わっていないことの確かめ

1. 分岐の先端 (全試作入り) で `pip install maturin highspy==1.15.1 && pip install -e .` (または `maturin develop --release`)。回した先端のコミットを README に書き足す。
2. `python analysis/protoscreen_20261124/check_default.py`: 学習・計測と重ならない P の 6 問 (air03・neos8・exp-1-500-5-5・qnet1・mik-250-20-75-4・markshare_4_0) を
   1 スレッド・種 1・上限 200 秒 (決定的) で解き、状態・ノード数・目的値を基準と比べる。基準は analysis/losses_20261123/res_pgocheck.jsonl の lto_v3 の回
   (40be041e4 のソース、今回の試作の多くが入る前。作り方は道筋を変えない)。「全部一致」を確かめてから回す。
   - 2026-11-24 に先端 7d54b3ced で回した: 6 問とも一致 (ノード 0・0・7・3・6333・236024)。試作は既定オフのとき既定の道筋を変えていない。
   - 分担 4: 先端 ed107692d で `pip install -e .` し、check_default.py で 6 問とも一致を確かめてから回した (2026-11-24 01:44 開始)。
   - 環境変数を渡すと (`python check_default.py ENOMOTO_MIP_POOL_SEL=1`)、その試作でどの問題から道筋が変わるかを見られる。

## 4. 集計 (summ.py)

試作ごとに、同じスレッドの数の既定と、両方そろった (問題, 種) で比べ、1 スレッドと 4 スレッドの表を分けて出す (計算は analysis/slowm_20261119/ci.py と同じ):
- 比: ずらし幾何平均の比 (ずらし 10 秒、解けない回は 60 秒)、問題ごとの種の平均から 95% 区間。解けた数 (既定 → 試作)、全種で 1.25 倍超の遅れの問題。
- 食い違い: 既定との最適値の不一致・最適と実行不能の衝突、MIPLIB の既知の値 (solu.json、58 問すべて) との不一致。
- 採否の目安: 区間の上端 <= 1.05・解けた数が減らない・全種 1.25 倍超の遅れなし・食い違い 0 の全部を満たせば「候補」、外れたら理由を書く。

## 5. 台本の確かめ (この環境、結果はコミットしない)

khb05250・p0201 × 21 設定 × 種 1、上限 10 秒 (`python run.py 0 確かめ用の分担表 結果`): 42 回とも結果が書かれ、crash 0、食い違い 0。summ.py の表も出た
(上限 10 秒・2 問なので比は動作の確かめだけ)。

## 6. 分担と回した記録

- 分担 0〜6: 親が Gm3w・HCoW・HJZ2・HsTC・JhRi・GcYE・Fnaw に割り当てた。分担 7: SCIP-VB (集計役) が回す。
- 分担 7: 先端 ed107692d (src は 7d54b3ced と同じ) を `maturin develop --release` で入れ、check_default.py で 6 問とも一致を確かめてから回し始めた (2026-11-24)。
- 12 番目の試作 (木の中の打ち切り値の判定に利用者・TPRE の打ち切り値も数える、HuKp が作成中) は、名前が届いたら別の小さな台本で P の 1・4 スレッドを足す
  (既定はこの計測の def_t1・def_t4 の行を共有)。

## 7. 12 番目の試作 (後から足した)

- 12 cutu: `ENOMOTO_MIP_CUTOFF_USER_TREE=1` (analysis/ti6_20261124、HuKp)。木の中の「打ち切り値に使える値があるか」に外側の求解の打ち切り値
  (利用者の `ENOMOTO_MIP_CUTOFF`・TPRE の予備の解の値・再スタートで引き継いだ値) も数える。1・4 スレッド。
- 分担表は別: `python plan12.py 2` → shards_c12.json (58 問 × 種 1〜3 × {cutu_t1, cutu_t4} = 348 回、60 秒、2 分担、各 1.59 時間)。
  回し方 `python run.py K shards_c12.json res_c12_sK.jsonl` (K = 0・1)。既定は shards.json の def_t1・def_t4 の行を共有する (同じ分担ではないので組ではない)。
- shards.json (1〜11 番) は作り直していない (分担 0〜7 はすでに回し始めている)。summ.py は res_s*.jsonl と res_c12_s*.jsonl の両方を読む。
- ビルド: この試作の入った先端で作り直してから回す (分担 7 を回している間は作り直さない)。
