# 既定候補を全部入れた組 (combo) を MIPLIB benchmark 240 問で測る (2026-11-24、利用者の依頼、親の割り当て)

利用者の依頼「既定候補を全部入れて、Mittelmann 186 問と MIPLIB 240 問を計測」。M の 186 問は MIPLIB 2017 の benchmark 集合 240 問に全部入るので、
240 問を 1 回回し、集計を「M 186 問」「240 問全部」「残りの 54 問」の 3 通りで出す。既定は変えない。新しい閾値は作らない。

## 1. 設定 (cfg.py)

| 名前 | 中身 | スレッド |
|---|---|---|
| def_t1・def_t4 | 既定 (ENOMOTO_ で始まる環境変数は全部外す) | 1・4 |
| combo_t1・combo_t4 | 既定 + 下の 8 つ全部 | 1・4 |

| 環境変数 | 試作 | 記録 |
|---|---|---|
| ENOMOTO_MIP_SB_PROBEBD=1 | sbpb (強分枝の両方の子の伝播から境界を置く) | treegap_20261123、protoscreen 11 |
| ENOMOTO_MIP_CUTOFF_USER_TREE=1 | cutu (木の中の打ち切り値の判定に外側の打ち切り値も数える) | ti6_20261124、protoscreen 12 |
| ENOMOTO_MIP_NODELP_BFRT_GROUP=1 | bfrt (木のノードの LP の比率テストの同点を組で) | lpiter_20261123、protoscreen 9 |
| ENOMOTO_MIP_PROOFPROP=i | ppi (2 スレッド以上で双対証明の本数に上限) | proofprop_20261123、protoscreen 10 |
| ENOMOTO_MIP_HEUR_ADAPT=1 | 1 スレッドの解探しの試作を割合つきで | heuradapt_20261124 |
| ENOMOTO_MIP_NODEROWS_ADAPT=t | ノードの行の管理を φ と手間あたりの伸びの比で | nodespeed_20261123 9.6 節 |
| ENOMOTO_MIP_ENUM_ADAPT=1 | 列挙の前処理とプロービングの繰り返しを割合つきで | cutgap_20261123 8 節 |
| ENOMOTO_MIP_CUTSEL_PROBE_TIE_FEWER=1 | カット選びの下見の同点で本数の少ない側 | shelved_20261122・cutrobust_20261124 |

- HEUR_ADAPT は中で `HEUR1T` の f・l、`FP_STAGE3`・`POOR_NOINC`・`P2RIT=a`・`FJSYNC_LABEL` を割合つきで使う (src/mip/solver.rs の `heur_adapt` の説明と
  heuristics.rs:1188・1243・1292、solver.rs:7749・11051・12090・14849・25476 で確かめた)。別の変数は要らない。
- ENUM_ADAPT は中で `ENUM_PRESOLVE=all`・`ENUM_WHERE=loop`・`PRR_TIGHT` を使う (mod.rs の `enum_presolve_mode`・`enum_where`)。

## 2. 問題 (problems.json・solu.json)

- all: https://miplib.zib.de/downloads/benchmark-v2.test の 240 問。M: analysis/cbcimpl_20261029/mitt_parts.json の 186 問 (全部 all に入る)。
  rest: 残りの 54 問 = holdout_20261123 の 53 問 + nw04 (P に入り M に入らない)。
- solu.json: https://miplib.zib.de/downloads/miplib2017-v31.solu から 240 問 (最適値 232・実行不能 7・最良の値 1)。holdout の 53 問は holdout の solu.json と同じ。
- MPS は run.py が holdout と同じ `fetch` で `.miplib_cache/` に取る (残りの 54 問は gz で約 524 MB、展開後はその数倍)。

## 3. 回す前の確かめ (2026-11-24、先端 ac89261c1、`pip install -e .`)

1. ビルド: 先端 ac89261c1 で `pip install -e .` (前回の P と同じ形)。
2. combo の 8 つを全部つけて fuzz 7 本と cargo test (`bash analysis/combo_20261124/fuzz.sh`): 誤答 0。
   fuzz_tiny 0/400・fuzz_highs 0/300・fuzz_objint 0/1000・fuzz_slack 0/1000 (mismatches 0)・fuzz_tiny2 0/400・fuzz_objint2 0/1000・fuzz_slack2 0/1000。
   cargo test 424 通過・0 失敗。
3. `python analysis/protoscreen_20261124/check_default.py`: 6 問とも基準と一致 (既定の道筋は変わっていない)。
4. `python analysis/combo_20261124/smoke.py` (p0201・misc07・noswot・nw04、1 スレッド・種 1・60 秒、既定・各変数だけ・combo):

| 問題 | 既定 | SB_PROBEBD | CUTOFF_USER_TREE | BFRT_GROUP | HEUR_ADAPT | NODEROWS_ADAPT | ENUM_ADAPT | TIE_FEWER | combo |
|---|---|---|---|---|---|---|---|---|---|
| p0201 | 0.9 秒・28 | 0.9・31 | 0.8・28 | 0.7・21 | 0.9・28 | 0.7・18 | 0.9・28 | 0.8・28 | 0.7・11 |
| misc07 | 5.1・4004 | 4.7・3535 | 5.3・4004 | 5.4・4034 | 6.9・4456 | 5.0・3974 | 5.2・4004 | 5.4・4004 | 9.6・5768 |
| noswot | 23.0・29898 | 20.2・24021 | 23.1・29898 | 8.7・16937 | 23.1・30006 | 29.2・55331 | 12.9・20135 | 22.9・29898 | 46.9・71093 |
| nw04 | 18.8・0 | 18.5・0 | 18.6・0 | 19.6・0 | 18.3・0 | 18.1・0 | 18.5・0 | 19.3・0 | 19.7・0 |

   (秒・ノード。1 回ずつで、時間は fuzz と同時に回したので揺れが大きい。道筋の違いはノード数で見る)
   - ログの行: SB_PROBEBD は子の両方から置いた境界 1・30・130 (p0201・misc07・noswot)、HEUR_ADAPT は `HEURADAPT root` (木のある 3 問)、
     NODEROWS_ADAPT は `node rows adapt` (4 問)、ENUM_ADAPT は `ENUM_ADAPT:` (p0201 1 行・misc07 4 行)。combo でも同じ行が出る。
   - CUTSEL_PROBE_TIE_FEWER: 診断 `ENOMOTO_MIP_CUTSEL_PROBE_TIE_LOG` で、nw04 の下見が同点で本数が 2 対 1 (`tie true ... cuts Some((2, 1))`) =
     TIE_FEWER が選びを変える場面 (nw04 はどちらでも根で解ける)。noswot は同点だが本数が同じ (5, 5) で変わらない。
   - CUTOFF_USER_TREE: 4 問とも道筋は既定と同じ (外側の打ち切り値 (利用者・TPRE・再スタート) がある問題でだけ働く。protoscreen では P で効き目あり)。
   - PROOFPROP=i: 2 スレッド以上でだけ働く (solver.rs の `proofprop`)。4 スレッドは決定的でないので、ここでは道筋で確かめていない。
   - 参考: noswot は combo で 23.0 → 46.9 秒 (NODEROWS_ADAPT だけで 55331 ノード)、misc07 は 5.1 → 9.6 秒。計測の前の 1 回で、判断は本計測で。

## 4. 分担 (plan.py → shards.json)

- 240 問 × {1, 4 スレッド} × {def, combo} × 種 1、上限 600 秒 = **960 回、11 分担、見積り各 10.72 時間 (計 117.9 時間)**。
- 同じ (問題, スレッド) の def と combo は同じ分担で続けて回す (組)。組の中の順は組ごとに入れ替える。
- 見積り: analysis/final_20261118 の既定の 1 回の実時間 (wall) を def と combo の両方に使う (M の 600 秒の行、nw04 は P の 60 秒の行)。
  ほかの問題 (残りの 53 問) は 630 秒 (上限 + 待ち) と置いた。見積りのある (問題, スレッド) は 374 / 480。残りの 53 問は 600 秒より短い問題も多いので、実際は短くなる側。
- 作り直しは `python analysis/combo_20261124/plan.py 11` (回し始めたら作り直さない)。

## 5. 回し方 (各分担)

```
git fetch origin ccr-1beed81b-ex0uhq && git merge --no-edit FETCH_HEAD
pip install maturin highspy==1.15.1 && pip install -e .        # 先端でビルド (計測の間は作り直さない)
python analysis/protoscreen_20261124/check_default.py            # 6 問とも一致を確かめる
python analysis/combo_20261124/run.py K                         # K = 0〜10 → analysis/combo_20261124/res_sK.jsonl
```

- 止めても (2 時間の上限など) 同じコマンドで続きから回る。済んだ (問題, 設定, 種) は飛ばし、crash・取得失敗の行は次の回にやり直す。上限 + 300 秒で打ち切り (killed)。
- 4 スレッドの回は 4 コアを使うので、同じ機械でほかの計測と並べない。ソルバーは同時に 1 つ。
- 計測の間は作業ツリーの `python/` を変えない (先端をマージすると、editable の .so と python/ の版がずれることがある。protoscreen の分担 7 の記録)。
  途中で push するときは、マージの後で python/ を回し始めの版に戻すか、マージせずに res_sK.jsonl だけをコミットして push する。
- 行には `commit` (回し始めの HEAD) が入る。集計の始めにコミットの内訳を出す。

## 6. 集計 (summ.py)

`python analysis/combo_20261124/summ.py > summ.txt`。スレッド別 (1・4) × 集合別 (M 186 問・240 問全部・残りの 54 問) に、
ずらし幾何平均の比 (ずらし 10 秒、解けない回は 600 秒) と 95% 区間 (問題の間の 1.96 × 標準誤差。種 1 つ)、解けた数と片方だけ解けた問題、
食い違い (combo と既定、既定・combo と solu.json)、1.25 倍超の遅れの問題と 0.8 倍未満の効いた問題の一覧 (時間・状態つき)。
目安は protoscreen と同じ (区間の上端 <= 1.05・解けた数が減らない・1.25 倍超の遅れなし・食い違い 0)。

## 7. 分担の組み替え (takeover.json・prog.py)

進みの偏りが大きくなったら、遅い分担の終わりの組を、早く終わった子へ渡す。既定の動き (takeover.json がないとき) はこれまでと同じ。

- 進みを見る: `git fetch` と取り込みのあと `python analysis/combo_20261124/prog.py`。分担ごとに、済んだ回数、実績の比、残りの見積り (時間)、渡した組の数を出す。
  - 実績の比は、済んだ回の wall の合計を、plan.py のその回の見積りの合計で割ったもの。
  - 残りの見積りは、残りの回の見積りにこの比を掛けたもの。
- 渡す: `takeover.json` に `{"K": N}` を書いて push する。分担 K の終わりの N 組を渡すという意味で、1 組は同じ (問題, スレッド) の def と combo の 2 回。
  - 分担は見積りの大きい順に並んでいるので、終わりの組は軽い組から渡ることになる。
  - 組は切れ目で割れない (確かめ済み)。def と combo は同じ子・同じ台で回る。
- 持ち主: 取り込んでから、同じコマンド `run.py K` で回し直す。takeover.json は始めるときに読むので、回し直すまで前の分け方のまま回る。
  - 渡した N 組は回さなくなる。
  - 2 時間の上限で止まったときの回し直しで、自然に新しい分け方に変わる。
- 手伝い: `python analysis/combo_20261124/run.py K --tail` → `res_sK_tail.jsonl`。止めても同じコマンドで続きから回る。
- 重なり: 持ち主も手伝いも、`res_sK.jsonl` と `res_sK_tail.jsonl` の両方を見て、済んだ回を飛ばす。
  - 渡す前に持ち主が回した組は、手伝いが回さない。
  - それでも同じ回が両方で回れば、summ.py が crash・取得失敗でない後の行を使う。
  - summ.py はもともと `res_s*.jsonl` を全部読むので、`_tail` の行も入る。
- 分担 1 本に手伝いは 1 つ。2 つ以上に分けたいときは、渡す組の数を手伝いの数で割って別の分担から渡す。

## 8. 回している途中で気づいたこと

- neos-3402454-bohle は、台のメモリ (15 GB) が足りないために回が無効になった。ソルバーの誤りではない。
  - 4 スレッド (分担 0):
    - def_t4 は 2 回とも、約 85 秒で OOM で殺された (returncode −9、RSS 13.4〜13.6 GB)。
    - combo_t4 は RSS 9.5 GB のまま上限 + 300 秒を過ぎ、打ち切った (killed)。
  - 1 スレッド (分担 10): def_t1 (RSS 9.8 GB) も combo_t1 (5.7 GB) も、上限 + 300 秒を過ぎて打ち切った。
  - 上限で止まらなかったのは、メモリが逼迫して遅くなったためとみられる。final_20261118 にはこの問題の行がない。
  - run.py は crash の行を、回し直しのたびにやり直す。
    - ただし外から −9 で殺された crash (OOM) が 2 回そろった回は、crash で確定してやり直さない (親の依頼、2026-11-24)。
    - rail02 の def_t4 (分担 5、RSS 13.6 GB、570 秒で OOM) のように、1 回に 10 分近く使う回を回し直しのたびに繰り返さないため。
    - prog.py も同じ数え方をする。
    - 上限 + 300 秒の打ち切り (killed) は、これまでどおり済んだ回として数える。
  - 集計では 2 つの形で出す。
    - ふだんの形: どちらも解けなかった回 (上限の秒) として数える。
    - 併記の形: def と combo の両方が無効な組を比の計算から外す。summ.py の表の「両方無効の組を除く」の行と、「両方とも記録が無効な組」の一覧。
- 分担 0 の途中で取り込んだ先端には、src の新しい試作が入っている。
  - そのため行の `commit` が 8e6cc38eb 以外になることがある。
  - 回しているビルドは ac89261c1 の `pip install -e .` のままで、作り直していない (src は 8e6cc38eb と同じ)。
