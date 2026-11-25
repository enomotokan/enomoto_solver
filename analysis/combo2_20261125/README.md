# 組み直した組み合わせ (combo2) の計測 (2026-11-25、親の依頼・利用者の承認)

前回の combo (analysis/combo_20261124) の最終の集計と害の切り分け (同 README 10.4 節・analysis/comboharm_20261124) を受けて、組み合わせを組み直す。
- nra・enum・heur は直しを入れた形にする。
- sbpb は外す (c2a)、SB_ADAPT の適応形にする (c2b)、常に使う (c2c) の 3 通りにする。
  c2c は親の追加の依頼 (利用者の問いを受けて)。

計測は 2 段。
- 第 1 段 (この記録): P (58 問 × 種 1〜3 × 60 秒、1・4 スレッド) で c2a・c2b・c2c を比べる。
- 第 2 段: 第 1 段の結果から、1 つを MIPLIB 240 問 (1・4 スレッド、600 秒) に回す。前回の combo と同じ台本を使う。

既定は変えない。どの設定も既定に足す試作の環境変数で、新しい閾値は作らない。

## 1. 設定 (cfg.py)

版: 先端 a5008f9d8。
- 磨きの既定化 6762d9c3c (POLISH_STRICT=2 + POLISH_WARM=d が既定) の後の版。r0k23・1s・1mk・FP_STAGE3_LS・LMIP_SCORE_TICKS が入っている。
- **既定と試作は同じ版で回す。**

| 設定 | 環境変数 (どれも既定に足す) |
|---|---|
| def | なし |
| c2a | 共通 c2 (下) |
| c2b | c2 + `ENOMOTO_MIP_SB_ADAPT=1` |
| c2c | c2 + `ENOMOTO_MIP_SB_PROBEBD=1` (強分枝の子の伝播を常に使う。前回の combo と同じ sbpb) |

共通 c2 の中身:

| 変数 | 値 | 中身 (前回の部品の名前) |
|---|---|---|
| NODEROWS_ADAPT | r0k23 | nra の直し: 外す側だけ (r0) + 判定 K と持ち越し (k2・k3)。comboharm 10.9 節 |
| ENUM_ADAPT | 1s | enum の直し: 1 周目の途中でも止める (時計を始めから数える手直し後)。comboharm 10 節 |
| HEUR_ADAPT | 1mk | heur の直し m (サブ MIP の中で FP の第 3 段を回さない) と k (f_t を手間の秒で数える) |
| FP_STAGE3_LS | a | FP の第 3 段を FJ2・Local-MIP で解く (analysis/fpstage3ls_20261125) |
| LMIP_SCORE_TICKS | 1 | Local-MIP の手間の数え違いの直し (app1-1、analysis/lmipticks_20261125) |
| NODELP_BFRT_GROUP | 1 | bfrt |
| CUTSEL_PROBE_TIE_FEWER | 1 | tie |
| CUTOFF_USER_TREE | 1 | cutu |
| PROOFPROP | i | ppi (木を分け合うときだけ働く。1 スレッドでは働かない) |

- c2a・c2b は SB_PROBEBD (sbpb) を置かない。c2c は置く。
- c2b の SB_ADAPT=1 は、強分枝の子の伝播を、`sbad_f` の f_B の確率でノードごとに使う適応形として働く。
  - src/mip/solver.rs の `sb_probebd = SB_PROBEBD がある || (SB_ADAPT で u < f_B)` で、SB_PROBEBD があれば常に使う。
  - ほかに、退化のノードでの強分枝・HiGHS 並みの予算 (f_A)、衝突の重みの床 (f_B) も変える (analysis/sbadapt_20261124)。
- 変数の読み方は cfg.py の先頭に書いた。どれも値を確かめた。
  - HEUR_ADAPT は `heur_adapt` が「0 でなければ有効」、`heur_adapt_flag` が「文字を含むか」なので、1mk で m と k の両方が入る。
  - NODEROWS_ADAPT の r0k23 は、comboharm で確かめた文字列そのまま。

## 2. 問題と分担 (plan.py → shards.json)

- 問題: P の 58 問 (analysis/cbcimpl_20261029 の PROBS、protoscreen と同じ)。solu.json は protoscreen の写し。
- 回数: 58 問 × 種 1〜3 × 8 設定 (def・c2a・c2b・c2c × 1・4 スレッド) = 1392 回、上限 60 秒。
- 4 分担: 0 が VB、1 が HJZ2、2 が JhRi、3 が HsTC。
  - 見積りは各 3.2 時間、計 12.7 時間。final_20261118 の P の既定の時間を試作にも使い、1 回に 8 秒の待ちを足した。
  - 同じ (問題, 種) の 8 回は同じ分担で続けて解く。順は組ごとに回して入れ替える (protoscreen の plan.py と同じ)。

## 3. 回す前の確かめ

2026-11-25、先端 a5008f9d8 の `pip install -e .`。

1. check_default (`python analysis/protoscreen_20261124/check_default.py`): 新しい基準 ref_default.json と 6 問とも一致。
2. fuzz 7 本と cargo test。`bash analysis/comboharm_20261124/fuzz.sh` に c2a・c2b・c2c の変数をつけて回した。
   - 出力は fuzz.out (c2a・c2b) と fuzz_c2c.out。`FUZZ_VENV=/usr/local` で、計測と同じ入れ方のビルドを使った。
   - 3 つとも誤答 0。
     - fuzz_tiny 0/400・0/400
     - fuzz_highs 0/300
     - fuzz_objint 0/1000・0/1000
     - fuzz_slack は mismatches 0 (1000・1000 回)
   - cargo test は 3 つとも 424 通過・0 失敗。
3. smoke (`python analysis/combo2_20261125/smoke.py` → smoke.out)。p0201・misc07・noswot・nw04・app1-1 を、1 スレッド・種 1・60 秒で回した。
   - 各変数のログの行が出る。
     - NODEROWS_ADAPT は `node rows adapt`、ENUM_ADAPT は `ENUM_ADAPT:`、HEUR_ADAPT は `HEURADAPT root`。
     - SB_ADAPT は misc07 で、強分枝の子の伝播 (`node bounds from both children`) を 4 ノードで使った (確率の形)。c2c (SB_PROBEBD=1) は noswot で 17。
   - FP_STAGE3_LS も効いている。
     - HEUR_ADAPT だけのとき、misc07 で FP の第 3 段はサブ MIP の形 (`FP stage 3: `) で 1 回回る。
     - HEUR_ADAPT + FP_STAGE3_LS では、サブ MIP の形が 0 回になり、LS の形 (`FP stage 3 LS`) が 2 回回る。
   - NODELP_BFRT_GROUP・SB_ADAPT・CUTSEL_PROBE_TIE_FEWER は、ノード数が既定と違い、道筋が変わる (効いている)。
   - LMIP_SCORE_TICKS・CUTOFF_USER_TREE は、この 5 問では道筋が既定と同じ。
     - LMIP_SCORE_TICKS は、暫定解のないサブ MIP の robust FJ でだけ当たる (lmipticks_20261125)。
     - CUTOFF_USER_TREE は、外から打ち切り値を与えたときだけ働く。
     - どちらも、読む所 (cfg.py の先頭) で値を確かめた。
   - PROOFPROP=i は、木を分け合う 4 スレッドでだけ働く (前回の combo と同じ)。
   - 注: app1-1 の c2a は 17.8 秒で、既定は 9.0 秒。
     - tie (CUTSEL_PROBE_TIE_FEWER) だけで 1201 ノード・13.6 秒になり、前回の切り分け (heur + tie、tie の根の道筋) と同じ向き。
     - c2b は 55 ノード・14.4 秒。


## 4. 回し方 (各分担)

1. ブランチを取り込み (`git fetch origin ccr-1beed81b-ex0uhq && git merge --no-edit FETCH_HEAD`)、先端で `pip install -e .`。
2. `python analysis/protoscreen_20261124/check_default.py` が「全部一致」になることを確かめる。基準は ref_default.json。
   - コンテナが再起動すると highspy が消えることがあり、そのとき check_default は IndexError で落ちる (分担 3 の報告)。
   - 始める前と回し直しの前に `python -c "import highspy"` で highspy があるかを確かめる。なければ `pip install highspy==1.15.1` で入れ直し、check_default を確かめ直す。
3. `python analysis/combo2_20261125/run.py K` (K は分担の番号) → `res_sK.jsonl`。
   - 止めても、同じコマンドで続きから回る。同じ (問題, 設定, 種) の行は飛ばす。
   - ソルバーは同時に 1 つで、ほかの計測と並べない。
   - 2 時間の上限で止まったら、同じコマンドで回し直す。計測の途中でビルドし直さない。python/ が変わる取り込みもしない。
4. `res_sK.jsonl` を push する。push の前に取り込む。

## 5. 集計 (summ.py)

`python analysis/combo2_20261125/summ.py [--perprob] > summ.txt` で、protoscreen と同じ形の表を出す。
- 中身: c2a・c2b ごと、スレッドごとに、比・95% 区間・解けた数・全種 1.25 倍超の遅れ・食い違い・効いた / 害の問題。
- 害の問題には、前回の切り分けの原因 (analysis/combo_20261124/cause.json) を `[前回: …]` で付ける。
- c2c と c2a の問題ごとの比べ (親の依頼): 比が 1.25 倍超か 0.8 倍未満の問題を、種ごとの比と並べて出す。
  - 全部の種で同じ向き (全部 > 1 か全部 < 1) なら「同じ向き」(仕組みの見込み)、種で向きが変われば「変わる」(運の見込み)。
  - 上限同士 (比 1) の種は数えない。
- 第 2 段に c2a・c2b・c2c のどれを回すかの材料にする。
