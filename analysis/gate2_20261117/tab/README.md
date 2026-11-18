# gate2 候補 1: TAB_HIGHS_RULES を作業量のシグモイドで入り切り (2026-11-17)

元: `ENOMOTO_MIP_TAB_HIGHS_RULES=1` (tableau の行の選び方を HiGHS の HighsTableauSeparator に)。58 問 P −3.3% [−10.5, +5.1]
(analysis/revisit_20261112/summ_P.txt)、揃って遅い問題 (nw04・pk1・sp150x300d・neos-960392) があり不採用。

## 1. 分ける特徴 (結果を見て選んだので楽観的)

- 材料: `ratio.py` = revisit_20261112 (種 1・2) と gap4_rootstuck_mem_20261109/res58hr (2 回) の 4 対の問題ごとの比 (tab / def)。
  `feat.py` = 58 問を根だけ (node_limit 1、60 秒、種 1、`ENOMOTO_MIP_DEBUG_SEP=1`) で既定・tab の 2 通り解いた特徴 (`feat.jsonl`、`logs/feat_*`)。
  `an.py` → `an.txt` (並べた表)。
- 大きさ・密さ (m・n・nnz/m・集約行の非零 / n) では分けられない (順位相関 |ρ| ≤ 0.18)。
- 作業量 **W = (根の最初の tableau の分離で既定の選び方が使った行の集約行の非零の合計) / (LP の非零 + 列)** が一番効く:
  - W ≥ 2.5: qnet1 0.59 (W 7.1)・10teams 0.71 (18.8)・neos-911970 0.77 (7.0)・rout 0.86 (5.9)・mc11 0.68 (2.6)・dcmulti 0.78 (2.7)・
    eil33-2 0.69 (2.8)・qiu 0.82 (2.6)・air03 0.88 (2.3)・physiciansched6-2 1.02 (5.4)。外せない遅い問題は binkar10_1 1.34 (3.7)・nubu 1.18 (3.7)。
  - W < 2.5 で外せる遅い問題: roll3000 2.06 (1.2)・nw04 1.93 (0.66)・mik-250 1.50 (0.98)・mas76 1.22 (0.84)・pk1 1.20 (1.3)・
    sp150x300d 1.16 (0.44)・nu25-pr12 1.15 (0.44)・neos-960392 1.14 (1.3)。取りこぼす速い問題: hypothyroid-k1 0.75 (0.01)・neos-827175 0.72 (0.12)・
    neos-860300 0.62 (0.56)・neos17 0.70 (1.2)。
- 別の害: HiGHS の fill の予算 (集約で増える非零の 10 倍 > 10000 + 列) で tableau 行が全部外れる問題 (fw = 予算を超える行の割合 = 1)。
  nw04 は既定では根の密な tableau カット (1 本 6 万非零) で閉じるが、tab ではカット 0 本 → 再スタート (根だけ 12.7 → 34.3 秒)。
  30n20b8 (W 21、fw 1) は根の下界 139.9 → 126.0。eil33-2・10teams も fw ≈ 1 だが tab で速い (分けられない)。

## 2. 試作 `ENOMOTO_MIP_TAB_HIGHS_SIG` (src/mip/separation.rs、既定では使わない)

- 根の最初の tableau の分離 (`light` でない呼び出しで行が 1 つ以上使われたもの) は既定の選び方で行い、W と fw を測る。
  σ((ln W − ln `ENOMOTO_T_MIP_TAB_SIG_W` (2.5)) / `ENOMOTO_T_MIP_TAB_SIG_WIDTH` (0.25)) ≥ 0.5 なら以後 (ノードを含む) HiGHS の選び方。
  fw ≥ `ENOMOTO_T_MIP_TAB_SIG_FW` (0.9) なら既定のまま (値 `n` なら HiGHS の選び方で fill の予算だけ外す)。
  決めた結果は `ENOMOTO_MIP_LOG=1` で `MIP: tableau rule sigmoid: W .. fw .. sig .. -> ..` と出る。再スタート・サブ MIP ではそれぞれ測り直す。
- 根だけの試し (種 1、`feat.jsonl` の札 sig1・sigd。sigd は今の `1`、sig1 は今の `n` に当たる):

| 問題 | 判定 | 既定 | tab | 試作 (`1`) | 試作 (`n`) |
|---|---|---|---|---|---|
| qnet1 | W 7.1 → HiGHS | 15945 2.5 秒 | 15967 2.3 秒 | - | 15859 1.6 秒 |
| roll3000 | W 1.2 → 既定 | 12498 18.4 秒 | 12497 24.6 秒 | - | 12498 20.6 秒 |
| nw04 | W 0.66 → 既定 | 16862 12.7 秒 | 16862 34.3 秒 | - | 16862 14.5 秒 |
| 30n20b8 | W 21・fw 1 | 139.9 45.0 秒 | 126.0 32.8 秒 | 139.9 46.2 秒 | 129.8 48.4 秒 |
| 10teams | W 19・fw 0.96 | 924 6.3 秒 | 924 3.7 秒 | 924 4.0 秒 | 924 5.4 秒 |
| eil33-2 | W 2.8・fw 1 | 822.5 9.2 秒 | 811.3 7.6 秒 | 822.5 9.5 秒 | 822.5 12.3 秒 |
| mc11 | W 2.6 → HiGHS | 11618 10.0 秒 | 11547 9.2 秒 | - | 11575 12.1 秒 |

  (`-` は `1` と `n` で同じ判定なので測っていない。根だけの時間は揺れが大きい)

## 3. 確かめ

- 誤答: `fuzz.sh` (release019_20261117/fuzz.sh の写し。しきい値を 0.01 に下げてほぼ全部で HiGHS の選び方を通す)。結果は下の 4 節。
- 対の比べ (しきい値を選ぶのに使っていない種): `cfg.py` (def・tsig)・`plan.py`・`run.py`・`summ.py` (slow2_20261117 の写し)。
  P = MIPLIB 58 問 × 種 4〜6 × 60 秒 (`shards_P.json`、3 分担 × 約 1.0 時間)、M = Mittelmann 48 問 × 種 3・4 × 600 秒
  (`shards_M.json`、8 分担 × 約 2.1 時間)。回し方: リポジトリの根で `sh analysis/revisit_20261112/build.sh` の後、
  `python analysis/gate2_20261117/tab/run.py P K` (K = 0〜2)、`... run.py M K` (K = 0〜7)。集計 `python summ.py P > summ_P.txt`。

## 4. 誤答の確かめ (2026-11-17)

- cargo test (`--release`): 419 通過・0 失敗・19 無視。
- `fuzz.sh` (`ENOMOTO_T_MIP_TAB_SIG_W=0.01`): fuzz_tiny 0/400・fuzz_highs 0/300・fuzz_objint 0/1000・fuzz_slack 0/1000・fuzz_tiny2 0/400、
  値 `n`: fuzz_tiny 0/400・fuzz_highs 0/150。誤答 0。

## 5. 代表問題の通し (60 秒、4 スレッド、種 7、1 回)

| 問題 | 既定 | tsig |
|---|---|---|
| qnet1 (W 7.1 → HiGHS) | 2.6 秒 | 2.3 秒 |
| 10teams (fw 0.96 → 既定のまま) | 7.4 秒 | 3.9 秒 (同じ判定なので揺れ) |
| roll3000 (W 1.2 → 既定) | 37.8 秒 | 34.0 秒 |
| nw04 (W 0.66 → 既定) | 14.6 秒 | 15.0 秒 |
| neos-911970 (W 7.0 → HiGHS) | 15.8 秒 | 15.4 秒 |

最適値はどれも一致。tab で揃って遅かった roll3000・nw04 は既定と同じ扱いになり害が消える。

## 6. p0201 の誤答と直し (2026-11-18)

- 最初の版 (v1) の P 分担 0・1 (種 4・5、`res_P_v1_s0.jsonl`・`res_P_v1_s1.jsonl`) で、tsig の p0201 種 5 が 7675 (最適 7615) を返した。
  手元の再現: `ENOMOTO_MIP_TAB_HIGHS_SIG=1` で 40 種中 13 回が 7675。既定は 40 回中 0 回、`TAB_HIGHS_RULES=1` は 30 回中 0 回、
  しきい値 0.01 (どこでも HiGHS の選び方) も 30 回中 0 回。
- 元: 主の根は W 1.60 で既定の選び方に決めたが、裏のカットの写し (`solve_worker` の写し) が自分で測り直して W 3.30 で HiGHS の選び方になり、
  主と写しの選び方が混ざった。デバッグ解 (前処理後の問題を HiGHS で最適に解いたもの、`mkdbg.py` の手順) で見ると、写しから戻ったカットの行も
  主の候補のカットもデバッグ解を切っていない。解を失ったのはノード 4 で、対称性の固定 (reason −5。WRONG のセッションの切り分けでは orbital fixing / SYMDYN のタグ 3 で、orbitope ではない) が列 10 を 0 に固定し、
  デバッグ解の最後の対称な像が消えた (`MIP_DEBUG_SOL lost at node 4 (main thread): local redcost fixing`)。
  WRONG のセッションの切り分け: SYMDYN=f (orbital fixing + LexRed の併用) で起き、どちらか一方だけでは起きない。1 スレッドでも出る。
  カットの選び方の混ざりは道筋を変えただけで、本体は既定の対称性の処理の組み合わせ (直しは WRONG の側)。
- 直し: 写しは主の決めた選び方を受け取る (`BG_TAB_SIG`、src/mip/solver.rs)。p0201 は 40 種中 0 回の誤答になった。cargo test 419 通過、
  `fuzz.sh` 7 本とも誤答 0。P は直した版で種 4〜6 を測り直す (`res_P_s*.jsonl`)。
