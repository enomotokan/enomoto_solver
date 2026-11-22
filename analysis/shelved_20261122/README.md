# 見送った試作から「結果を変えない直し」を拾い直す (2026-11-22、親の依頼)

問い: 見送った試作のうち、(1) 正しさ (誤答・誤った衝突・数値の保険)、(2) 頑健さ (止まる・LP の失敗・crash の回避)、(3) 資源 (メモリ・期限の後の超過)、
(4) 再現性 (決定性・時刻への依存) の直しで、計測が中立 (区間が 1 をまたぐ・解けた数が減らない・全種の遅れなし) か害が見えないのに見送られたものはあるか。
(5) 探索の道筋を変えるもの (カットの選び方・止め方・ヒューリスティクスなど) は対象外。

調べた範囲: analysis/revisit_20261112/inventory.tsv・inventory_abc.md (745 件)、docs/backlog.md、docs/improvement_history.md の見送り・不採用、
analysis/ の 2026-11-12〜22 の記録 (182 件)。4 つの補助のエージェントに範囲を分けて読ませた。上位の候補は、今の src に環境変数があることと
既定になっていないことを自分で grep して確かめた。

## 結論 (先に)

- **(1)〜(4) の直しの大半は、すでに既定に入っている。** 例: MEM* 一式、BOUND_STEP、FZ3_REPAIR_BOUND、SUBMIP_HALT、ZH/POLISH_DEADLINE、
  ROOTLP_SCALE、BG_CUTS_DETERM、TIMEDEP=b・DET1T、CUT_SMALL_FLOOR・FINISH_CANCEL、TIGHTEN_INTBOUND、SUBMIP_FEASTOL、
  RESTART_LPFAIL_NOCUTS、INHPOOL・SEP_TAB_STREAM、期限の直し 4 つ (FEAS_EARLY・DSE・DIVE・ENTRY)、DETHASH・TICKS。
  条件にきちんと当たる見送りは少なく、**はっきりした候補は 3 つ** (A〜C)。ほかは道筋も変える「境界」のもの。
- **A. `ENOMOTO_MIP_OVERRUN3=ild` (資源: 期限の後の超過)**: 期限の後か、求解の後にだけ働く 3 つの直し。時間内の探索の道は変えない。
  - 記録 (analysis/mittelmann_ready_20261103/overrun.md) は「既定にしてよい候補、58 問・Mittelmann で 1 回確かめてから」とし、その後の計測がない。
  - 60 秒で 1 秒を超える超過は 18 回中 6 → 4 回 (`zil`)。300 秒の radiationm40 は 4.7 → 1.0 秒 (`zilhd`)。時間内に解ける 8 問の最適値は同じ。
  - 同じ記録の `z` は `ENOMOTO_MIP_ZH_DEADLINE` として既定になったが、残りの文字は置き去りになっている。
- **B. `ENOMOTO_MIP_FZ2_FIX=1` (正しさ: 前処理の丸めの余裕を許容誤差に揃える)**
  - 整数行の正規化で、右辺の切り下げの余裕を 1e-9 から 1e-6 (`INT_ROUND_TOL`) に揃える。今は、許容誤差の内にある点 (違反 2e-9 など) を前処理が切りうる。
  - 記録 (analysis/fuzz_tiny_worse_20261103.md) は「MIPLIB で前処理の結果が変わるかを見てから」として未計測。当時の数字は、悪い値を最適と返す件 5 → 0、HiGHS との食い違い 13 → 9。
  - 今日の既定で fuzz を回し直した (checks/): fuzz_tiny 400 (種 0) の厳しい判定は 10 → 9、種 1000 は 12 → 12、bad はどちらも 0 (今の判定)。mip_fuzz_vs_highs 300 は bad 0。
  - 当時の誤答の多くは、ほかの直しで消えている。残る効きは小さいが、向きは緩める側 (許容誤差の内の点を切らなくなるだけ)。
- **C. ipmxm (頑健さ: 根の LP の内点法 + クロスオーバーが頂点を返さない・仕上げで止まる失敗の直し)**
  - 一式は `ENOMOTO_XO_MIP_CLEANUP_HINT=1`・`ENOMOTO_XO_IPM_RETRY_PC=3`・`ENOMOTO_XO_NO_BOUND_STOP_ZERO_COST=1`・`ENOMOTO_IPM_KKT_AUG_RATIO=4`。
  - 58 問 × 2 回で SGM +0.7%・解けた 89 → 88 (timtab1 の 1 回)・p ≥ 0.17・超過 0。mc11 の害は除いた版。
  - 「Mittelmann 600 秒で比べてから」のまま止まっている (analysis/ipmx_harm_20261106.md)。
  - 根の LP の頂点が変わりうるので、道を変える面はある。効くのは大きな LP だけ (CLEANUP_HINT は列 + 非零 5 万以上)。
- **測り直しの分担表を用意した** (割り当ては親)。def と比べる、4 スレッド。
  - P: 58 問 × 種 1〜3 × {def, ov, fz2, ipmxm}、60 秒、6 分担 × 約 0.94 時間 (`shards_P.json`)。
  - M: 186 問 × 種 1 × {def, ov, ipmxm}、600 秒、20 分担 × 約 2.75 時間 (`shards_M.json`)。fz2 は効く問題が少ない見込みなので、M から外した。
  - 回し方: `cd analysis/shelved_20261122 && python run.py P k` (k = 0..5)、`python run.py M k` (k = 0..19)。
  - 集計: `python ci.py 60 def ov res_P_s*.jsonl` など (設定ごとに def と対で)。ov は超過も `python overrun.py res_*.jsonl` で比べる。
- 判断の目安 (提案): ov は「時間が中立 (区間が 1 をまたぐ) で、超過の 1 秒超の回が減る」なら既定に。fz2 は「時間が中立で、最適値の食い違い 0」なら
  正しさの直しとして既定に。ipmxm は M で中立以上なら既定に。

## P の測り直しの結果 (2026-11-22、6 分担を別の子が回した。ビルド 25562ff71、4 スレッド・60 秒・種 1〜3)

- `python ci.py 60 def X res_P_s*.jsonl` (`ci_P.txt`)。どれも中立で、全回で 1.25 倍超の遅れはない。crash 0、最適値の食い違い 0 (各分担の報告)。

  | 設定 | SGM X/def (95% 区間) | 解けた def / X |
  |---|---|---|
  | ov (OVERRUN3=ild) | 1.007 (0.980〜1.034) | 138 / 137 |
  | fz2 (FZ2_FIX=1) | 1.003 (0.985〜1.021) | 138 / 138 |
  | ipmxm | 1.006 (0.988〜1.024) | 138 / 139 |

- ov の解けた数 −1 は 10teams 種 3。def は根で解けて 2.7 秒・0 ノード、ov は時間切れ。fz2・ipmxm も同じ回が木に入って 33.5・10.8 秒。
  ov は期限の後にしか働かないので、10teams の揺れ (根で解けるか) と見る。
- 期限の超過 (`overrun.py` → `overrun_P.txt`): 60 秒の時間切れの回 (35〜37 回) は、def も含めて全設定で結果の時刻の超過が平均 0.03 秒・最大 0.2 秒、
  1 秒超は 0 回。壁時計の超過 0.6 秒は、起動・読み込みと計測の台本の待ちの分。
  - 記録の当時 (2026-11-03) に 60 秒で 18 回中 6 回あった 1 秒超の超過は、その後の期限の直し (ZH_DEADLINE など) で、今の既定ではもう出ない。
  - **P の 60 秒では、ov の効く場面がない**。ov の効き目 (300〜600 秒の radiationm40・uccase12 などの超過) を見るには M が要る。
- 時間切れの回の暫定解は、どの設定も def と良し悪しが混じる (各分担の報告)。
- 判断の案:
  - **fz2**: 中立で、解けた数・食い違いも同じ。正しさの揃え (許容誤差の内の点を切らない) として既定にしてよい。ただし fuzz で見える効きは今は小さい (strict 10 → 9)。
    → **既定にした** (親・利用者の判断、ef2c07e92。`ENOMOTO_MIP_FZ2_FIX=0` で戻す。docs/improvement_history.md に節)。M は fz2 を含まない最初のビルドのまま回す。
  - **ipmxm**: P は中立 (解けた +1、mzzv42z 種 3)。狙いは大きな根の LP なので、M で確かめてから決める。
  - **ov**: P では害も得もない (効く場面がない)。M で超過が減るかを見る。

## M で悪化した問題の切り分け (利用者の指示「ぶれが大きいので、既定より悪くなっている問題は調べるべき」、`det/`)

方法: まず「その設定が本当に計算を変えているか」を分ける。
- 1 スレッド・種 1・ノード 200 で打ち切り、def・ov・ipmxm を回す。比べたのは決定的な数え: 根の LP の値・反復、カットのラウンドの反復・目的値、
  根の止まり方・手間 (CUTSTOP の tk)、終わりのノード数・下界・暫定解、進みの行の lp_iters・sb_iters・node_iters・heur_iters・dive_iters。
  台本は `det/det.py`・`det/summ.py`、結果は `det/det.txt`。時刻で出る進みの行・時刻・メモリは比べない。
- 4 スレッドの根 (ノード 1、種 1〜3、def) で、主の根の LP の同時実行の勝ち手を見る (`ENOMOTO_MIP_XOLAZY_LOG` の winner)。
  台本は `det/race.py`、結果は `det/race.txt`。ipmxm は内点法 + クロスオーバーが根の LP を解く場面でしか働かない。

結果:
- **1 スレッド・決定的には、11 問すべてで ov・ipmxm とも既定と同じ道** (数えが一致)。対象の 11 問: neos-911970・neos5・neos-1171737・
  neos-1456979・gmu-35-40・graphdraw-domain・brazil3・csched007・blp-ar98・b1c1s1・leo2。
  - **ov**: 期限の後にしか働かない作りのとおり、期限の前に道は変わらない (不具合はない)。
    M で ov が遅れた・悪化した回 (gmu-35-40 1.75 倍、neos-1171737 2.25 倍、neos5 1.32 倍、graphdraw-domain 1.33 倍など) は、4 スレッドの揺れ。
  - **ipmxm**: 1 スレッドの根の LP は同時実行をしないので、内点法が出てこない。
- **4 スレッドの根の LP で内点法が勝つのは、csched007 (種 1〜3 とも) と neos5 (種 2 だけ) だけ**。
  ほかの 9 問は単体法か、前の基底からの warm が勝つ。この 9 問では ipmxm も計算を変えず、M の差は揺れ。
- 計算が変わる 2 問を、4 スレッドの種 2・3 で def と ipmxm で回し直した (`det/full4.py` → `det/full4.jsonl`):

  | 問題 | 種 | def (状態・秒・根の LP の勝ち手・根の LP の反復) | ipmxm |
  |---|---|---|---|
  | neos5 | 2 | 最適 34.5 秒・simplex・64 | 最適 41.9 秒・ipm・63 |
  | neos5 | 3 | 最適 29.7 秒・simplex・64 | 最適 43.6 秒・ipm・64 |
  | csched007 | 2 | 時間切れ (351 / 下界 341)・ipm・1024 | **最適 401.6 秒**・ipm・704 |
  | csched007 | 3 | 時間切れ (352 / 324)・ipm・1280 | 時間切れ (351 / 326)・ipm・960 |

  - neos5: 根の LP は 63 行 × 63 列で、数ミリ秒で解ける。ipmxm では種 2・3 とも同時実行の勝ちが内点法に替わり、頂点が変わって 1.2〜1.5 倍遅い (M の種 1 も含め、そろった遅れ)。
    ipmxm は内点法の失敗を直すためのものだが、neos5 には直す失敗がない。小さな LP で勝ち手が入れ替わる副作用。
  - csched007: 内点法が毎回勝つ。ipmxm ではクロスオーバーの後の根の LP の反復が少ない (1024 → 704、1280 → 960)。種 2 は解けた (M の種 1 は csched007 を def も ipmxm も時間切れ)。
- 判断の材料:
  - ov の悪化は揺れで、ov 自体の害は見つからない。
  - ipmxm は、内点法の勝つ少数の問題でだけ働き、得 (csched007) と損 (neos5) が混じる。

## 候補の表

確度: ◎ 条件に当たる、○ 境界 (道も変える・未計測)、△ 弱い (害の兆し・効きが薄い)。「src」は今の src の grep (2026-11-22、HEAD 付近)。

| 確度 | 環境変数 (値) | 分類 | 何を直すか | 見送った理由と数字 (記録) | src | 害がないと言える根拠 | 測り直すなら |
|---|---|---|---|---|---|---|---|
| ◎ | `ENOMOTO_MIP_OVERRUN3=ild` | 3 | i: 期限後に受け取った解の twoopt などを省く。l: 期限後は双対単体法の破綻の後の解き直しをしない。d: 求解の後の待ち行列 (数 GB) の解放を裏のスレッドへ | 「既定にしてよい候補」のまま bench が回らず。60 秒の 1 秒超の超過 18 回中 6 → 4 (zil)、radiationm40 300 秒で 4.7 → 1.0 秒 (zilhd)、時間内に解ける 8 問の最適値は同じ (mittelmann_ready_20261103/overrun.md) | ある: zerohalf.rs:61 (`overrun3`)、solver.rs:5977 (d)・7706 (i)、slope_intercept_dual.rs:4309 (l)。既定ではない | 3 つとも期限を過ぎた後か求解の後にだけ働く (コードの条件 `past`・求解の後)。時間内の道は変えない。変わりうるのは、期限の後に受け取った解の磨きの分だけ (時間切れの回の暫定解がわずかに悪くなりうる) | P・M で def と対。時間の比、時間切れの回の超過 (overrun.py)、時間切れの暫定解 |
| ◎ | `ENOMOTO_MIP_FZ2_FIX=1` (`n` 行の正規化・`m` 併合した整数列の上限) | 1 | 前処理の整数への丸めの余裕を 1e-9 から 1e-6 (`INT_ROUND_TOL`) に揃え、許容誤差の内の点を切らない | 「MIPLIB で前処理の結果が変わるかを見てから」で未計測。当時 fuzz_tiny 400 で悪い最適 5 → 0、HiGHS との食い違い 13 → 9、mip_fuzz 0/300 (fuzz_tiny_worse_20261103.md)。今日: strict 10 → 9 (種 0)・12 → 12 (種 1000)、bad 0 → 0 | ある: mod.rs:9281 (`fz2_fix`)。既定ではない | 右辺を緩める向きの丸めだけ (他の整数の丸めと同じ許容誤差)。注意 (推測): 行を最大公約数 g で割った後の 1e-6 は、元の尺度では g × 1e-6 の違反を許しうる | P で def と対 (時間・解けた数・最適値の食い違い)。fuzz 3 種 |
| ◎ | ipmxm: `ENOMOTO_XO_MIP_CLEANUP_HINT=1`・`ENOMOTO_XO_IPM_RETRY_PC=3`・`ENOMOTO_XO_NO_BOUND_STOP_ZERO_COST=1`・`ENOMOTO_IPM_KKT_AUG_RATIO=4` | 2 | 根の LP の内点法 + クロスオーバーが最適な頂点を返さない・仕上げが長引く失敗を直す (内点法だけで解いたとき 11 問中の最適な頂点 5 → 9) | 「60 秒では得が見えない、Mittelmann 600 秒で比べてから」で止まった。58 問 × 2: +0.7%、解けた 89 → 88、p ≥ 0.17、超過 0、dano3_3 時間切れ → 最適 (ipmx_harm_20261106.md、rootlp_ipm_fail_20261105.md) | ある: crossover.rs:904・1015・2612、kkt.rs:1830。どれも既定ではない | 目的値は同じで、変わるのは頂点の選び方 (対象 11 問で二段解法と一致)。頂点が変わるので道は変わる | M (600 秒) と P で def と対 |
| ○ | `ENOMOTO_MIP_OVERRUN3=hr` | 3 | h: 根の局所探索の時間の枠を期限までの残りで頭打ち。r: 期限の直前 (残り max(1 秒, 根の LP の 2 倍) 未満) に再スタートしない | ab2 の 1 秒超の超過 11 回中 2 → 0 (zilh)、uccase12 の再スタートの超過を r で直す (overrun.md) | ある: heur_scip.rs:601、solver.rs:23419 | 期限の直前だけ振る舞いを変える。際の問題で揺れうる (記録の注意) | A の後に ov+hr を P で |
| ○ | `ENOMOTO_T_XO_IPM_RETRY_PDLP=500` | 2 | 内点法が受理できない点で終わったときだけ、PDLP の点から解き直す (二段解法への差し戻しの代わり) | 109 問で得がない (1.018 倍、退行なし)。MIP の根の LP では未計測 (pdlg_20261111.md、backlog 490〜496 行) | ある: crossover.rs:1052 (tunable、既定 0) | 失敗のときだけ働くので、成功する LP には影響しない | 根の LP の失敗例 (rootlp_ipm_fail の 11 問) で |
| ○ | `ENOMOTO_MIP_PRBWORK=1` (2 版目) | 3 | 成果 0 のまま手間の上限で打ち切られたプロービングを、再スタートの後に繰り返さない (含意は引き継ぐ) | 得が小さいとして広い計測なし。1 スレッド irp 39.3 → 35.9 秒、働いたのは 203 問中 3 問、fuzz bad 0 (probework_20261120 §2〜6) | ある: mod.rs:7525・8585 | 引き継ぐ含意は前の問題を狭めたものなのでそのまま成り立つ。省く回はプロービングをしないので道は変わる | P で対 (働く問題が少ないので優先度は低い) |
| ○ | `ENOMOTO_XO_BUMP_LU_RATIO=1` | 3 | クロスオーバーの押し出しの LU に双対単体法と同じ fill の上限を付ける (cod105 の再分解 65 回 14.6 秒 → 1 回 0.48 秒) | 4 スレッドで PXO +1.7% [−1.8, +6.6]・141 → 140、MXO +18.4% [−0.2, +33.3]。1 スレッドでは −0.3%・−9.0% (scipsym_20261120 §13) | ある: crossover.rs:3407 | 効くのは nnz(LU) ≥ 64m の密な分解だけ。Netlib 93 問は同じ道 | M の 4 スレッドの遅れの原因の切り分けから |
| ○ | `ENOMOTO_MIP_ROOT_BASIS_FIX_BIG=1` | 2 | 前処理の LP の基底を根に渡すとき、行が変わっていれば作り直して特異な基底を避ける (非零 20 万以上の LP だけ) | P・M 未計測。neos-873061 の根の LP 72 → 10 秒、fuzz 誤答 0 (highsloss_20261113.md (b)) | ある: solver.rs:1926、mod.rs:6494 | LP の最適値は同じ。頂点が変わるので道は変わる | M で対 |
| ○ | `ENOMOTO_MIP_ROOTLP3=s` | 2 | 最初の根の LP に渡した基底が特異なら捨てて解き直す (既定の `e` は再スタート後の根だけ) | +0.8% [−3.1, +5.5]、解けた 83 → 84、誤答 0、「効きがない」(rootlp3_20261111) | ある: rootlp3.rs:22 (既定 `e`) | 特異な基底のときだけ働く。出番はほぼない | 低優先 |
| ○ | `ENOMOTO_MIP_TREE3_DIVECAP=1` (・`_BGCAP`・`_CLIQUECAP`) | 3 | 根の Pump・ダイビング・裏のカットの差し替えの LP・clique の試しの LP に、根の LP の秒に比べた時間の上限を付ける (puriri が 607.3 秒で返った) | 「600 秒向け」で単独の計測なし。t3 一式の 58 問は 42 → 43・SGM 19.16 → 19.12 (tree_close3_20261104.md) | ある: solver.rs:4455・11802、separation.rs:948 | BGCAP は超えたら元のカットに戻るだけ。DIVECAP は予算の上限で道は変わる | M で対 |
| ○ | `ENOMOTO_MIP_CUT_FAIL_RESTORE=1` | 2 | カットを足した LP が失敗したとき、基底だけでなく LU・DSE 重みも戻して解き直す | 揺れの範囲 (21 問 −2.1%・40 問 +1.5%、cbcimpl_20261029.md) | ある: separation.rs:2028、solver.rs:11770 | 失敗のときだけ働くが、DSE 重みが変わるので後の道は変わりうる | 低優先 |
| ○ | `ENOMOTO_MIP_CUTSEL_PROBE_TIE_FEWER=1` | 4 (5 寄り) | カットの選び方の下見で伸びが同点のとき、時間あたりの伸び (時間で揺れる) ではなく足したカットの少ない側を選ぶ | −0.6% [−6.7, +5.0]、88 → 88、誤答 0。「PARALLEL_INT を採らない以上入れる理由はない」(parint2_20261113.md §5) | ある: separation.rs:2507 | 同点のときだけ働く。TICKS の `c` が既定になった今は動機が薄い | 低優先 |
| △ | `ENOMOTO_MIP_TREE_NO_PASS_BASIS=1` | 3 | 共有の待ち行列に出すノードに親の基底を付けない (widden の最大 RSS 4330 → 3167 MB) | 1 問 1 回だけ (gap4_rootstuck_mem_20261109) | ある: solver.rs:2201 | 受け取ったノードの温め始動がなくなり道が変わる | メモリの重い M の問題で |
| △ | `ENOMOTO_MIP_GAP4=rocnl` / `ENOMOTO_MIP_ROOTCAP=d` | 3 (5 寄り) | 密なカットの非零に上限 (rail01 の最大 RSS 4052 → 2662 MB) | rail01 だけ、下界は同程度 (candmem_20261121 §5) | ある | カットを変えるので道は変わる | 対象外に近い |
| △ | `ENOMOTO_SIMPLEX_LUCLOCK=b` | 4 | 共有の下界の BTRAN を LU の合成クロックから外す | +2.9% [−0.3, +6.6]、82 → 81 (luclock_20261111.md) | ある: basis_kernel.rs:887 | 区間はほぼ害の側 | 測り直さない |
| △ | `ENOMOTO_SIMPLEX_WARM_SING_REPAIR=3`・`ENOMOTO_MIP_ROOT_BASIS_FIX=s` | 2 | 特異な warm start の基底を列の差し替えで直す・根の基底が特異になる元を直す | 58 問で解けた数が −1〜−2 (singular_fix_a/c_20261102.md) | ある | 害の兆し | 測り直さない |
| △ | `ENOMOTO_XO_MIP_SCALE=1` 一式 | 2 | 尺度の悪い根の LP (momentum1) を尺度を揃えた写しで解く | 10teams が 3 回中 2 回遅い (原因未確認) (rootlp_ipm_fail_20261105.md §11) | ある (一部 `DUAL_MEGIDDO_RACE_NOLIMIT` は src にない) | 根拠が弱い | 10teams の切り分けから |

除いたもの (補助のエージェントの報告から):
- 害が見えたもの: PROBE_REPRESOLVE・ROBUST_STALL=rw (誤答の芽)、FACE_RESCUE (解を失う)、DSE_WEIGHT_CHECK (解けた数 −2)、NOISE_RESET (supportcase42)、
  LPPROBE_TICKS=w (解けた数 −1)、P2BASIS (M の上限 +11%、実時間の判定)、DSE_COST (wolgan の反復の爆発)、TPRE=plg (遅い)、ORDINV (+3.1%)。
- 道を変えるもの (5): ROBUST_ROOTIPM・ROOTLP_BOTH・SUBCAP・NODELP_BFRT_GROUP・SEP_ROUND_IMPLINT など。
- 環境変数のない案: 疎行列の添字を u32 の SoA にする変更 (2026-09-27、メモリ −25%、速さ中立で取り消し。コードは履歴 2c489f4..2310dcd にだけある)。

## 台本と生データ

- `cfg.py`: 設定 (def・ov・fz2・ipmxm)。`plan.py`: 分担表 (`python plan.py P 6 def,ov,fz2,ipmxm`、`python plan.py M 20 def,ov,ipmxm`)。
- `run.py`・`ci.py`: orbpp_20261121/wide の写し (置き場所の深さだけ直した)。`overrun.py`: 時間切れの回の超過の比べ。
- `checks/`: FZ2_FIX の有無の fuzz (今の既定で)。
