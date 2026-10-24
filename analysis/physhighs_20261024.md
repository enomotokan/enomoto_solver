# physiciansched6-2: HiGHS はどうやって最適解 49324 を得ているか (2026-10-24)

- 機械: 4 CPU。ソルバーは同時に 1 つだけ。HiGHS 1.15.1 (highspy)。ソースは GitHub の `ERGO-Code/HiGHS` タグ `v1.15.1`
  (`highs/mip/HighsPrimalHeuristics.cpp`・`HighsMipSolverData.cpp`・`HighsMipSolver.cpp`) を読んだ。
- 台本・ログは `analysis/physhighs_20261024/`。ブランチ `inv/physprimal-20261024`。

## 1. 結論

事実:

1. **HiGHS の暫定解は 1 つだけで、それが根の RENS のサブ MIP (`L`) が出した最適解**。
   - 既定 (4 スレッド): 根の LP 47410.76 (4.4 秒) → カットで 49324 (9.5 秒)、その間に暫定解はなく (BestSol = inf、FJ も解なし)、
     12.7 秒に `L` が 49324 を出して 1 ノードで最適 (`logs/highs_def.log`)。
   - オプションで切ると (60 秒、`highs_opt.py`):

     | | 最初の解 | 60 秒 |
     |---|---|---|
     | 既定 | `L` 49324 (12.8 秒) | 最適 12.8 秒 |
     | `mip_heuristic_run_rins=false` | `L` 49324 (12.5 秒) | 最適 12.5 秒 |
     | `mip_heuristic_run_rens=false` | 分岐 (`B`) 49325 (43.0 秒) | 時間切れ 49325 (下界 49324) |
     | 両方 false | 同上 | 同上 |

     RENS を切ると HiGHS も 60 秒で解けない。
   - 前の調査の、HiGHS を ENOMOTO の形 (前処理なし) にかけた回も、唯一の解は `L` (30.4 秒)。
2. **HiGHS の RENS の手順 (ソース)**: 根の LP の分離のループの後に、カットを含む LP の解から呼ぶ。
   - 整数列を lock の数の積の大きい順 (`setupIntCols`) に見て、今の LP 解の `floor`・`ceil` に定義域を狭める (LP で整数値の列は固定)。
     1 列ごとに伝播し、矛盾したらその段を打ち切る。
   - 1 段で固定するのは残りの自由な列の約 1 割 (`stopFixingRate = 1 − (1 − 今の率) × 0.9`)。**段ごとに LP を解き直し**、次の段は新しい
     LP 解で決める。LP が実行不能・打ち切り値以上なら 1 段戻る (10 回まで)。
   - LP で整数値の列がなくなったら、分数の列を `calcFixVal` (根の LP 解から 0.4 以上動いた向き、なければ目的が悪くなる向き、
     目的が 0 なら四捨五入) で、変化の和が 0.5 になるまで固定する。
   - 固定率が目標 (最初は 0.6、以後は成功・失敗の記録から乱数) に届いたら、その LP (カット入り、効いていない行を除く)・基底・境界で
     サブ MIP を解く。上限は葉 500・ノード 200 + ・停滞 12 ノード、時間は全体の残り、打ち切り値は外の上限。
   - ENOMOTO の既定の RENS (LP で整数値の列を全部まとめて固定、LP の解き直しなし) との違いは、(a) 固定を目標の率で止める、
     (b) 段ごとに LP を解き直して固定の値を選び直す、(c) サブ MIP にカットを渡し時間を制限しない、の 3 つ。`physlpdist` の
     「指示の列 z」(LP で 0 に見えるが分数の x が支えている値) は、x を固定すると LP の解き直しで z の値が変わるので、(b) で避けられる。
3. **ENOMOTO にも同じ形の RENS がある (`ENOMOTO_MIP_RENS_DIVE`) が、近傍は作れてもサブ MIP が解を出せない**。
   - 150 秒・AGG・再スタートなしで根の下界 49224〜49247 の後に `RENS_DIVE` + `SUBMIP_CUTS` + `SUBMIP_CUT_AGG`、サブ MIP に 30 秒:
     固定率 0.60・戻り 0 回・自由な列 2737・カット 1304 本、サブ MIP は 30 秒・17 ノードで解なし (`dive150_1`)。
   - 同じ形の回 (`dive150dump_1`) のサブ MIP を書き出す (`r_0.mps.gz`、自由な整数列 2171) と、
     - HiGHS (1 スレッド): 2.57 秒・1 ノードで最適 49424 (この近傍の最適。全体の最適 49324 は入っていない)。
     - ENOMOTO (サブ MIP の形、`SUBMIP_CUT_AGG`): 下界は 49424 に着くが 60 秒・850 ノードで解なし。
     - ENOMOTO (通常の求解、1 スレッド): 最適 49424 だが最初の解は 49.8 秒 (246 ノード)。
4. **HiGHS はその近傍のサブ MIP の解を、根のカットの後の乱択丸め (`R`) で出す** (3.1 秒、入れ子のサブ MIP なし)。
   - HiGHS は、サブ MIP の中か暫定解がないときは、**根の分離の各ラウンドの後に** 今の LP 解で乱択丸めを 1 回行う
     (`HighsMipSolverData.cpp` の `rootSeparationRound`)。丸めは lock の積の大きい順に、上向きの lock がなければ切り上げ、下向きが
     なければ切り捨て、ほかは `floor(x + U(0.1, 0.9))` で固定し、1 列ごとに伝播。矛盾したらやめる。最後に連続列の LP。
   - ENOMOTO はカットのループの後に 1 回 (3 回の試し)。同じ近傍で、丸めの順を HiGHS と同じ lock の順にし
     (今回の試作 `ENOMOTO_MIP_RR_HIGHS=lock`)、試しを 20 回 (`ENOMOTO_T_MIP_RR_TRIES=20`) にしても、カットの後の LP 解 (49424) から
     20 回とも矛盾で失敗 (1 回 0.002 秒)。
   - 前処理の後の大きさ: HiGHS 2657 行 × 1538 列、ENOMOTO 3425 行 × 自由な列 1989。

推測:

- HiGHS の暫定解の出どころは「段ごとに LP を解き直す RENS で近傍を作る」→「サブ MIP の根でカットが LP を近傍の最適値まで締める」→
  「各ラウンドの LP 解の乱択丸め (伝播つき) が当たる」の 3 段。ENOMOTO は 1 段目は同じ形を持つ (既定では使わない) が、3 段目が当たらない。
  当たらない理由は、ENOMOTO の丸めがループの後の 1 点だけで試すこと、前処理が弱く伝播で決まる列が少ないこと (行・列が HiGHS の
  1.3 倍) のどちらか、または両方とみる (切り分けていない)。
- 次に試すなら、(1) サブ MIP の根のカットの各ラウンドの後に乱択丸めを入れる (HiGHS と同じ)、(2) `r_0` で ENOMOTO と HiGHS の丸めを同じ
  LP 解から比べ、どの列の固定で矛盾するかを見る。

## 3. 各ラウンドの乱択丸め (追記、試作 `ENOMOTO_MIP_RR_EACH_ROUND`)

- 中身: サブ MIP の中か暫定解がないとき、根のカットの各ラウンドの LP 解で乱択丸めを `ENOMOTO_T_MIP_RR_EACH_ROUND_TRIES` (1) 回
  (`src/mip/separation.rs` `root_cut_loop`)。`ENOMOTO_MIP_RR_HIGHS=lock` と合わせて使う (lock の順・1 回目から乱数の閾値)。
- 書き出した近傍 4 つ (サブ MIP の形・1 スレッド・30 秒・`SUBMIP_CUT_AGG`):

  | 近傍 | 打ち切り値 | なし | `RR_EACH_ROUND` + `RR_HIGHS=lock` | HiGHS |
  |---|---|---|---|---|
  | `r_0` | 49525 | 解なし | 解なし | 49424・2.6 秒 |
  | `av_3` | 49824 | 解なし | 最適 49324・4.8 秒 | 49324・2.0 秒 |
  | `d1_3` | 50425 | 最適 49424・3.8 秒 | 49624 (時間切れ) | 49424・1.7 秒 |
  | `d2_3` | 50425 | 最適 49324・6.8 秒 | 解なし | 49324・3.1 秒 |

- **各ラウンドの乱択丸めは 1 回も当たっていない**。解が出た回はどれも Feasibility Pump (`av_3` の 4.4 秒、`d1_3`・`d2_3` の「なし」の回)
  で、乱数の使い方が変わって Pump の当たり外れが入れ替わっただけ。
- 失敗の中身 (`r_0`、`ENOMOTO_MIP_DEBUG_RR` に足した出力、`logs/r_0_rr_dbg_1.log`): どのラウンドでも、lock の順に自由な整数列
  (約 2000) のうち 48〜155 本目で、LP で整数値 (目標 0.000) の列を 0 にしても 1 にしても伝播が矛盾する。

推測:

- 丸めの手順 (順・閾値・伝播) は HiGHS と同じにしたので、違いは丸める LP 解の方にある。ENOMOTO のラウンドの LP 解は、LP で整数値の列の
  並びが整数解と両立しない (`physlpdist` の「指示の列」と同じ形) ので、どの順で丸めても早く矛盾する。HiGHS の LP 解 (別の前処理・
  別のカット) はそうなっていないとみる (HiGHS の LP 解は取り出していないので未確認)。
- **(追記) この推測は `physpool_20261024.md` の確かめで支持されなかった** (HiGHS のカットを足した LP 解の方が全部固定で実行不能。丸めが当たらないのは、罰の連続列を通して目的値が 50 万〜69 万に跳ねるため)。
- 確かめるには、HiGHS のカットプール (`physcmp` の `highs_trace.py DUMP_CUTS=1`) を近傍に足した LP の解を作り、同じ丸めが当たるかを見る。

## 2. 台本・ログ (`analysis/physhighs_20261024/`)

- `highs_opt.py <mps> <秒> [キー=値 ...]`: HiGHS をオプションつきで解く。`logs/highs_{def,norens,norins,nosub}.log`。
- `run_cfg.sh`・`eno_run.py`・`sumlog.py`・`highs_sub.py`: 前の調査と同じ。
- `logs/physiciansched6-2_dive150_1.log`・`dive150dump_1.log`: ENOMOTO の `RENS_DIVE` の回。
- `r_0.mps.gz`: `dive150dump_1` の根の RENS のサブ MIP。`logs/r_0_*.log` はその解き直し (`as_sub`・`full`・`as_sub_rrh`・`as_sub_rrlock`)。
- 試作 `ENOMOTO_MIP_RR_HIGHS=lock`・`ENOMOTO_T_MIP_RR_TRIES` (`src/mip/heuristics.rs` `randomized_rounding`)、`ENOMOTO_MIP_RR_EACH_ROUND`・`ENOMOTO_T_MIP_RR_EACH_ROUND_TRIES` (`separation.rs`)、`ENOMOTO_MIP_DEBUG_RR` の矛盾の位置の出力。どれも既定は不変。
- §3 のログは `logs/{r_0,av_3,d1_3,d2_3}_rr_{base,each}_1.log`・`logs/r_0_rr_dbg_1.log`。近傍の MPS は `r_0.mps.gz` と `../physprimal_20261024/subgz/`。
