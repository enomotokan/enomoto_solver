# 目的を外したダイビングとダイビングの反復の配り方の試作 (2026-11-21)

analysis/primalgap_20261121/README.md の直し案 2 (SCIP の objpscostdiving に当たるダイビングと、閾値のないダイビングの反復の配り方) を試作し、既定と比べた記録。

**結論: どちらも既定より良いとは言えない。既定は変えず、試作は環境変数で既定オフのまま残す。広い P/M の計測は行わない。**

## 試作 (コミット 5463b43dd、どちらも既定オフ)

- `ENOMOTO_MIP_OBJDIVE=1`: 木のダイビングの種類に `DiveKind::ObjPscost` (src/mip/heuristics.rs `obj_pscost_dive`) を加える。
  - 目的を `w*c/max|c| + push` に変えた LP を解き直す。w は 1 から始め、一歩ごとに `ENOMOTO_T_MIP_OBJDIVE_DEPTHFAC` (0.5) 倍する。
  - 一歩で、分数の列のうち pseudocost の点の上位 `frac/ENOMOTO_T_MIP_OBJDIVE_MULTI_DIV` (10) 本を押す。押す向きの目的の項は ∓2^k で、同じ列を押すたびに倍にする。
  - 次のときは上下限を固定して伝播する:
    - 押す向きの上下限が無限;
    - 押した回数が `ENOMOTO_T_MIP_OBJDIVE_PUSHES` (8) に達した;
    - 逆向きに選ばれ、かつ `ENOMOTO_MIP_OBJDIVE` に `f` を含む (含まなければ押す向きを反転する)。
  - 固定で LP が実行不能になったら、その一歩を取り消し、固定を 1 本ずつ両側で試す。
  - LP 解が整数になったら元の目的に戻して `try_lp_solution` に渡す。
- `ENOMOTO_MIP_DIVEQUOTA=s|e`: 木のダイビングの反復を、全体の上限 (既定 `0.05×(succ+1)/(calls+1)×node_iters+1000`) でなく種類ごとの割り当てで配る (src/mip/solver.rs)。
  - `s` (SCIP heur_dive 風): `(1+10*(succ+1)/(calls+1))*0.05*node_iters+1000`
  - `e` (使った反復あたりの成功): `(succ+1)/(iters/node_iters+1)*0.05*node_iters+1000`
  - 回す順で、反復が割り当てに届いていない最初の種類を選び、残りを予算にする。
  - 0.05 は `ENOMOTO_T_MIP_DIVEQUOTA_FRAC`。

## 計測

`python analysis/objdive_20261121/run.py A|B` で計測した (300 秒、4 スレッド、ビルドは 5463b43dd を merge した 956d0f6f3 で `pip install -e .`)。

- 設定:
  - def = 既定
  - dqs = DIVEQUOTA=s
  - dqe = DIVEQUOTA=e
  - ods = OBJDIVE=1 + DIVEQUOTA=s
- セット:
  - A = kakapo・brazil3・momentum1 × seed 1,2
  - B = 暫定解の遅い 5 問 × seed 1
- 結果は res_A.jsonl・res_B.jsonl、ログは logs/。
- 計測 A は 2 時間の制限で 23/24 本で一度止まり、残り 1 本を同じコマンドで足した。

### 最終目的値 (最小化)

| 問題 | seed | def | dqs | dqe | ods |
|---|---|---|---|---|---|
| academictimetablesmall | 1 | 3500 | 3500 | 3500 | 3500 |
| brazil3 | 1 | 239 | 207 | 216 | 237 |
| brazil3 | 2 | 231 | 234 | 237 | 216 |
| cost266-UUE | 1 | 2.52355e+07 | 2.52546e+07 | 2.52228e+07 | 2.52771e+07 |
| dws008-01 | 1 | 39712.6 | 41096.8 | 41397.2 | 41348 |
| momentum1 | 1 | 224674 | 527463 | 231074 | 231070 |
| momentum1 | 2 | 160483 | 192695 | 224663 | 192695 |
| neos-5107597-kakapo | 1 | 10800 | 5184 | 5139 | 12600 |
| neos-5107597-kakapo | 2 | 7893 | 13968 | 13833 | 10323 |
| rococoB10-011000 | 1 | 20053 | 21027 | 20140 | 21401 |
| traininstance2 | 1 | 77270 | 75070 | 76580 | 76980 |

既定との比 (幾何平均、11 組):
- dqs: 1.075 (良 3・悪 7)
- dqe: 1.013 (良 4・悪 6)
- ods: 1.062 (良 3・悪 7)

### 主スレッドの LP 反復に占めるダイビングの割合

| 問題 | seed | def | dqs | dqe | ods |
|---|---|---|---|---|---|
| academictimetablesmall | 1 | 1.2% | 42.7% | 22.7% | 50.0% |
| brazil3 | 1 | 0.8% | 48.1% | 20.6% | 52.6% |
| brazil3 | 2 | 1.1% | 53.1% | 21.2% | 57.4% |
| cost266-UUE | 1 | 0.5% | 20.6% | 22.0% | 23.7% |
| dws008-01 | 1 | 0.2% | 24.1% | 22.4% | 24.7% |
| momentum1 | 1 | 3.5% | 0.0% | 0.0% | 0.0% |
| momentum1 | 2 | 2.1% | 25.5% | 14.6% | 30.2% |
| neos-5107597-kakapo | 1 | 0.3% | 5.6% | 4.7% | 11.9% |
| neos-5107597-kakapo | 2 | 0.5% | 19.4% | 17.1% | 8.1% |
| rococoB10-011000 | 1 | 0.9% | 30.3% | 18.4% | 40.7% |
| traininstance2 | 1 | 0.3% | 4.5% | 23.3% | 26.9% |

## 事実

- 割り当てを変えると、ダイビングの反復の割合は既定の平均 1.0% から dqs 25%・dqe 17%・ods 30% に増えた。
- それでも、ダイビングが暫定解を出したのは 44 本中 5 回だけだった (incumbent の src に "diving" を含むもの)。内訳は dqs 2 回・ods 3 回で、既定と dqe は 0 回。
- 新しいダイビング (ods) は計 1107 回走った。整数の点に届いたのは 3 回、受け入れられたのは 1 回 (momentum1 seed 1) で、最終値には効いていない。
- kakapo と brazil3 は seed による違いが設定による違いより大きい。
  - kakapo の def は 10800/7893、dqs は 5184/13968。
  - 暫定解はほとんど tree proximity・tree rounding・Local-MIP から来ている。
- momentum1 seed 1 では、dqs・dqe・ods の 3 つとも 300 秒でノード 0 のまま終わった。
  - ルートのヒューリスティック "root reduced cost" の sub-MIP (500 ノード上限) が、dqs で 268 秒かかった (既定は 168 秒)。
  - dqs の 527463 はこのためで、試作の悪い面にあたる。
- dws008-01・rococoB10-011000 は 3 つとも既定より少し悪い。traininstance2 は 3 つとも少し良い。どれも 1 本ずつの計測。

## 推測 (確かめていない)

- momentum1 の遅れの理由: sub-MIP にも同じ環境変数が効き、sub-MIP のノードでダイビングが増えて遅くなったのだと思う。本採用するなら sub-MIP では無効にする必要がある。
- ダイビングに反復を回しても解が出ないのは、LP の緩さによると思う。kakapo ではルートの下界が SCIP 1209 に対しこちらは 190〜325 で、LP 解の分数の列が 1100〜1300 本と多い。そのため、どの順で丸めても実行可能な点に届かないのだと思う。主案の効きどころはダイビングの配り方よりルートの強さ (切除平面) にある、という見立て。
- 増えたダイビングの反復は、木のノードや他のヒューリスティックから時間を奪うので、平均では少し悪くなる。

## 誤答の確かめ

`bash analysis/objdive_20261121/fuzz.sh` で確かめた (OBJDIVE=1 + DIVEQUOTA=s で rootgap と同じ 7 本、cargo test は ods・objdive+dqe・既定の 3 形)。

(fuzz の結果は追って書き足す)
