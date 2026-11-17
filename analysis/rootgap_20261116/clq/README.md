# クリークのカットの自動の判定の見直しの確かめ (rootgap の 4 節)

設定は `cfg.py` の 3 通り (どれも環境変数だけで、ソースの既定は変えない):
- c18: CAND1118。クリークのカットは既定の `fallback`: ループの後に 1 回だけ足して、その伸びをループ全体の伸びの半分と比べる。
- c18_clqroot: + `ENOMOTO_MIP_CLIQUE_CUTS=root` (根でいつも回す)
- c18_clqmarg: + `ENOMOTO_T_MIP_CLIQUE_FB_MARGINAL=5` + `ENOMOTO_T_MIP_CLIQUE_FB_SECOND_ROUNDS=10`
  (fallback の判定の相手を、ループの最後の 5 ラウンドの伸びにする。使うときに続ける 2 回目のループは 10 ラウンドまで)

分担 (`python plan.py 4 8`。同じ (問題, 種) の 3 設定は同じ分担で、順は種で回す):
- P: 58 問 × 種 1〜4、60 秒、8 分担 (見積り各約 44 分): `python run.py P K` (K = 0〜7)
- M: mzzv11・mzzv42z・irp・hypothyroid-k1・tbfp-network × 種 1・2、600 秒、4 分担 (見積り各約 40 分): `python run.py M K` (K = 0〜3)

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy==1.15.1
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/rootgap_20261116/clq/run.py P K     # または M K
```

終わったら `res_<P|M>_sK.jsonl` (とログがあればそれも) をコミットして push する。集計は `python summ.py P > summ_P.txt`・`python summ.py M > summ_M.txt`。

## 1. P・M (3 設定) の結果 (`summ_P.txt`・`summ_M.txt`)

- P (58 問 × 種 4、60 秒): 解けた数 c18 179・root 177・marg 179 (232 組)、食い違い 0、crash 0。
  root / c18 = +0.8% [−5.7%, +8.8%] (noswot が全種で 5.1 倍遅、10teams 0.45・physiciansched6-2 0.78)。
  marg / c18 = +3.4% [+0.1%, +7.4%] (悪い)。
- M (5 問 × 種 2、600 秒): 解けた数 c18 8・root 9・marg 8。root / c18 = −1.7% [−23.8%, +27.1%] (mzzv11 0.64、irp は全種で 1.52 倍遅)。

## 2. シグモイドで割合を連続に決める形 (`ENOMOTO_MIP_CLIQUE_CUTS=sigmoid`・`sigmoid_a`、試作)

使う・使わないの 2 択でなく、根の点数 s から f = 1 / (1 + exp(−a (s − b))) を求め、根の 1 ラウンドの clique カットを round(f × 100) 本までにする
(0 本なら使わない。ノードでは使わない)。s は根の LP 解での 4 つの [0, 1] の量の平均
(`src/mip/separation.rs` の `clique_sig_score`):
- sp: 集合詰め込み・分割の形の行の割合
- ed: 衝突グラフの辺の数 / 2 値列の数 (ln(1 + 辺/列) / ln 101、100 で 1)
- fr: 分数の 2 値列のうち衝突グラフに辺を持つものの割合
- vi: 破れる clique カットの数 (最大 100) / 100

`sigmoid_a` (適応): 根の 3 ラウンド目の始めに、破れる clique カットだけを 1 回入れた LP の伸び g_c を直前のラウンドの伸び g_r と比べ
(どちらも LP を 1 回解く仕事)、z = a (s − b) + β ln((g_c + ε) / (g_r + ε)) で f を決め直す (ln は ±5 で切る、β = 1)。

a・b の合わせ方 (`sigfit.py` → `sigfit.txt`):
- `sigfit_run.py`: 58 問 + mzzv11・irp・tbfp-network を c18 (fallback) と root で根まで (node_limit 1) 解き、根の下界・時間と s を `sigfit.jsonl` に。
  根の下界の差での回帰は a = 3.16・b = 0.44 (当たり 9/15。差が出たのは 15 問だけ。多くは根で解けるか差が 1e-4 未満)。
- 1 節の P・M の結果の時間の比 root / c18 (問題ごとの幾何平均) で y = 1 (< 0.87)・0 (> 1.15) とした回帰は a = 1.79・b = 0.41 (当たり 8/12)。
  4 つの量ごとでは fr が最もよく (9/12) 、sp は逆向き (irp・mzzv42z は sp が大きいのに root で遅い)。
- **s と得の関係は弱い**: root で速いのは qiu (s 0)・neos-960392・drayage-25-23・neos-911970・physiciansched6-2・10teams・mzzv11 (s 0.35〜0.78)、
  遅いのは noswot (s 0.01)・pk1 (0)・qnet1 (0.32)・irp (0.57)・mzzv42z (0.78)。時間の比で合わせた a = 1.8・b = 0.41 を使う
  (f は s = 0 で 0.32、s = 0.8 で 0.67 と緩い。本数を半分程度に絞る形に近い)。

比べ (P2・M2。`cfg.py` の c18_sig・c18_siga だけを回し、c18・root は 1 節の P・M の結果を使う。別の機械の結果を並べるので機械の差が混ざる):
- P2: 58 問 × 種 1〜4、60 秒、4 分担 (見積り各約 59 分): `python run.py P2 K` (K = 0〜3)
- M2: 5 問 × 種 1・2、600 秒、2 分担 (見積り各約 52 分): `python run.py M2 K` (K = 0・1)
- 集計: `python summ.py P2 > summ_P2.txt`・`python summ.py M2 > summ_M2.txt`

tbfp-network は根の下界ではなく、解の見つけ方と被約費用の固定による再スタートの差 (rootgap の README の 2.3 節)。別の調べとして記録だけ残す
(s 0.44、根の下界・時間は fallback と root で同じ)。

## 3. P2・M2 の結果 (`summ_P2.txt`・`summ_M2.txt`。c18・root は 1 節の P・M、別の機械)

- P2 (58 問 × 種 4、60 秒、232 組): 解けた数 c18 179・root 177・sig 176・siga 177、食い違い 0、crash 0。
  - sig / c18 = −2.2% [−7.7%, +2.5%] (10teams は全種で 0.37。noswot は 1.18 で root の 5.1 倍からは大きく減る。neos-860300 1.27・neos5 1.23・pk1 1.19)
  - siga / c18 = −1.5% [−5.4%, +2.1%]、sig / root = −2.9% [−8.8%, +1.5%] (noswot 0.35)、siga / sig = +0.6% [−1.8%, +3.7%]
- M2 (5 問 × 種 2、600 秒): 解けた数 c18 8・root 9・sig 10・siga 9、食い違い 0、crash 0。
  - sig / c18 = −15.6% [−25.7%, −4.1%] (mzzv42z は全種で 0.69、mzzv11 0.74、tbfp-network 0.87。irp 1.0 前後で root の 1.52 倍は出ない)
  - siga / c18 = −5.9% [−17.3%, +8.6%] (irp 1.23)、siga / sig = +11.5% [+2.6%, +21.1%]
- 最大 RSS: sig は M で 3109 MB (c18 2725 MB)。
- まとめ: 固定の a・b のシグモイドは、P で中立〜わずかに良く (CI は 0 をまたぐ)、M で良い (CI が 0 をまたがない、ただし 5 問)。
  root の大きな負け (noswot・irp) をほぼ消し、root の勝ち (10teams・mzzv11) を残す。適応 (siga) は sig より良くない。
  c18・root と別の機械の結果を並べているので、次は同じ機械で c18 と sig を並べて確かめるのがよい。

## 4. 同じ機械での c18 と c18_sig の確かめ (P3・M3)

3 節の c18・root は 1 節と別の機械の結果で、P2 の分担 0 (種 1 が多い) は optimal 83 と他の分担 (90) より少なく、sig だけが落とした
eil33-2・neos-911970・roll3000・timtab1 (いずれも種 1、c18 は 27〜47 秒) は機械の差の疑いがある。c18 と c18_sig を同じ分担で続けて解く
(`python plan.py 2 4 3`、同じ (問題, 種) の 2 設定は同じ分担で、順は種で入れ替える):
- P3: 58 問 × 種 1〜4、60 秒、4 分担 (見積り各約 59 分): `python run.py P3 K` (K = 0〜3)
- M3: 5 問 × 種 1・2、600 秒、2 分担 (見積り各約 52 分): `python run.py M3 K` (K = 0・1)
- 集計: `python summ.py P3 > summ_P3.txt`・`python summ.py M3 > summ_M3.txt`

結果 (`summ_P3.txt`・`summ_M3.txt`):
- P3 (232 組): 解けた数 c18 178・sig 177、食い違い 0、crash 0。sig / c18 = −0.3% [−3.8%, +2.6%] (中立)。
  遅: neos-860300 1.21 (4 種とも 1.14〜1.26)・timtab1 1.19・neos-827175 1.18。速: 10teams 0.58・pk1 0.87。
  種 1 で落とした 4 問 (同じ機械): eil33-2 は c18 39.7 秒 / sig 時間切れ (sig の落としは残る)、neos-911970 25.2 / 24.3、
  roll3000 は両方時間切れ、timtab1 47.2 / 52.3 → 3 問は機械の差、eil33-2 の種 1 だけが sig の負け。
- M3 (10 組): 解けた数 c18 9・sig 10、食い違い 0、crash 0。sig / c18 = −8.0% [−20.0%, +6.4%]。
  mzzv11 は全種で 0.73 (c18 は種 1 で時間切れ、sig は 459・402 秒)、mzzv42z 0.86、tbfp-network 0.86〜0.94、irp 0.97 / 1.46。
- まとめ: 同じ機械で比べると、P は中立 (P2 の −2.2% は機械の差を含んでいた)、M は mzzv11・mzzv42z で速いが CI は 0 をまたぐ。
  sig は既定 (fallback) を悪くしないで mzzv11 を解くが、P で明らかに良くはない。
