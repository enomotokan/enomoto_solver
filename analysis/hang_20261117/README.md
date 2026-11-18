# HANG: 時間の上限を守らない処理と、SCIP が数秒で実行不能を示す 2 問 (2026-11-18)

依頼 (親セッション): analysis/vsscip_20261117/res_M_s31.jsonl (ビルド a9d7bae3d、Mittelmann 600 秒) で
1. fhnw-binpack4-4 の def_t1 (1 スレッド・種 1) が 600 秒で止まらず外側の 900 秒で kill (returncode -9)。
2. 同じ問題と neos859080 を SCIP 10 は 12.3 秒・2.3 秒で infeasible と示すが、当方は def・def_t1 とも 600 秒で決着しない。
追加: cryptanalysiskb128n5obj16 の def_t1 が 600 秒の上限で 608.8 秒 (res_M_s21.jsonl)。

- 機械: 4 CPU のコンテナ。土台: このブランチの 95240fe7b (a9d7bae3d より後)。SCIP は pyscipopt 6.3.0 同梱の 10.0.2。
- 台本: `one.sh 問題 スレッド数 種 秒 名前 [環境変数=値 ...]` (1 回をログつきで、`logs/<問題>_<名前>_s<種>.log`)、
  `watch_at.sh 問題 スレッド数 秒 開始秒 名前` (起動から開始秒を過ぎたら gdb で全スレッドのスタック、analysis/overrun_crypt_20261101 の写し)。
  対の比べ: `run.py`・`plan.py`・`summ.py`・`cfg.py` (analysis/slow2_20261117 の写し、6 節)。
- ログ: `logs/` (SCIP は `*_scip.log` = 統計つき)。fuzz の結果は `fuzz/`。

## 1. fhnw-binpack4-4 の 1 スレッドが止まらない: 原因 (事実)

- HEAD のビルドで再現: 1 スレッド・種 1・600 秒で 915 秒たっても終わらず kill (`logs/fhnw-binpack4-4_head_t1_s1.log`)。
- gdb (623 秒以降): 主のスレッドは `feas_fj_lmip_early_salt` の中の `LocalMip::run` (呼び手は根の後のやり直し `robust_fj_retry`、
  `ENOMOTO_MIP_ROBUST` の `g` = 既定)。
- 仕組み:
  - 1 スレッドの既定 (`ENOMOTO_MIP_TICKS` の `f`) では、FJ・Local-MIP の早めの探索の上限を**手間** (`crate::ticks`) で数え、時刻は
    「cap の 10 倍」の安全網だけにしていた。**前処理の期限 (= 求解の時間の上限) も見ない** (決定性のため、と注記)。
  - 根の後のやり直しは、実行可能性問題 (目的が全部 0) では cap = 残りの 0.5 (秒の上限なし)。根の時点で残り約 310 秒 → cap 155 秒 → 安全網 1550 秒。
  - しかも手間の数えが実時間よりずっと小さい (2 節): 前処理の早めの探索は cap 60 秒で 289 秒回り、やり直しは安全網まで止まらない。
- 4 スレッドでは早めの探索は時刻で止まる (cap 6 秒・12 秒どおり) ので、def は 600 秒で正しく止まる。
- neos859080 の 1 スレッドでも同じ型: やり直し (cap 6 秒、目的あり) が 46.4 秒回っていた (`logs/neos859080_def_t1_s1.log`)。

## 2. 手間の数えのずれ (事実)

診断のログ `MIP:   early FJ/Local-MIP (1 thread): FJ … (… in ticks), Local-MIP … (… in ticks)` を足して測った (実時間 / 手間の秒):

| 問題 | FJ | Local-MIP (直す前) | Local-MIP (連鎖の手を数えた後) |
|---|---|---|---|
| neos859080 (やり直し、cap 6) | 0.15 / 0.47 | 46.09 / 5.53 (8.3 倍) | 6.55 / 5.54 (1.2 倍) |
| fhnw-binpack4-4 (前処理、cap 60) | 2.4 / 5.4 | 261 秒 (内訳なし) | 126 / 54.6 (2.3 倍) |
| cryptanalysiskb128n5obj16 (前処理、cap 60) | 26.2 / 50.8 | — | 16.0 / 9.3 (1.7 倍) |

- Local-MIP の連鎖の手 (`localmip::chain_move`) の手間は `ENOMOTO_MIP_TICKS` の `j` (手間の時計、既定外) のときしか数えていなかった。
  既定の `f` の上限の中では連鎖の手が丸ごと数えに入らず、neos859080 では実時間の 1/8。
- 連鎖の手を数えても、Local-MIP の 1 手の手間 (`step_work` = (行の平均の長さ + 列の平均の長さ) × 見る列の数、固定の見積り) が
  binpack4-4 では実時間の 1/2.3 (長い行がある問題で平均が小さく出る)。FJ は逆に 2〜3 倍多めに数えている。

## 3. 直し (既定に入れたもの)

期限の直し 3 つは「期限 (時間の上限) を過ぎたときだけ道筋が変わる」もの。連鎖の手の数えだけは既定の 1 スレッドの道筋を変える。

1. `feas_fj_lmip_early_salt` (src/mip/mod.rs): 手間の上限 (`f`) の安全網の中でも前処理の期限を守る (`t_end = min(期限, t0 + 10 cap)`)。
   `robust_fj_retry` (src/mip/solver.rs) は求解の期限 (`start + time_limit`) を `with_presolve_deadline` で渡す。
   切り分け用に `ENOMOTO_MIP_FEAS_EARLY_NO_DEADLINE` で直す前と同じ。
2. 早めの探索の 1 スレッドの手間の上限の中では、連鎖の手の手間も数える (`localmip::count_chain_guard`、`ENOMOTO_MIP_FEAS_EARLY_NO_CHAIN_TICKS` で数えない)。
   他の道 (木・サブ MIP の Local-MIP) の数えは変えていない。
3. cryptanalysiskb128n5obj16 (4 節): `DseState::from_basis` (厳密な DSE の重み、行ごとの BTRAN) が 32 行ごとに期限 (単体法の時刻の上限・
   スレッドの期限・取り消し) を見て、過ぎたら残りの行は単位重みで返す (`ENOMOTO_DSE_NO_DEADLINE` で見ない)。
   ダイビングの `dive_try` は求解の期限を過ぎていたら LP を解き始めない (`ENOMOTO_MIP_DIVE_NO_DEADLINE` で見ない)。

## 4. cryptanalysiskb128n5obj16 の 1 スレッドの超過 (事実)

- 直す前の再現: 600.45〜604.3 秒 (gdb で止めた分を含む)。1 つ目の直しの後も 600.83 秒。
- 期限の直後のスタック (`logs/cryptanalysiskb128n5obj16_watch2_t1.gdb`、行 54770 の LP): ダイビング (`dive` → `dive_try`) の LP が
  期限の後に基底を作り直し (LU 分解)、`DseState::from_basis` → `dse_exact_row_sparse` で行ごとの BTRAN を 2.5 秒以上 (期限を見ない)。
  直した後 (`watch3`) は、片方の向きの LP が期限で止まった後に、もう片方の向きの LP を LU 分解から始めていた。
- 直した後 (`watch4`): 600.53 秒 (「solve returned TimeLimit after 600.51s」)。残る 0.5 秒は、この大きさの LP の反復 (約 2 ミリ秒) を 64 回ごとに
  期限を見る粒度と、LP を止めた後の後始末と見られる (推測、スタックは期限の前のものしか取れていない)。
- 608.8 秒 (vsscip) は 4 本が同時に回る計測機で、上の DSE の重みの計算・解き直しが遅くなった分と見られる (推測)。

### 4.1 cryptanalysiskb128n5obj14 の 1 スレッド (706.9 秒、親からの追加の材料その 2)

- 直しを全部外した形 (`ENOMOTO_MIP_FEAS_EARLY_NO_DEADLINE=1 ENOMOTO_MIP_FEAS_EARLY_NO_CHAIN_TICKS=1 ENOMOTO_DSE_NO_DEADLINE=1
  ENOMOTO_MIP_DIVE_NO_DEADLINE=1`、`logs/cryptanalysiskb128n5obj14_old_t1.*`) で再現: **713.1 秒**・0 ノード。期限の後のスタックは 10 回とも
  `with_presolve_deadline` → `feas_fj_lmip_early_salt` (根の後のやり直し FJ)。fhnw-binpack4-4 と同じ型 (1 節)。
  - 前処理の早めの探索: cap 60 秒で 134 秒 (Local-MIP 107.5 秒を手間 9.4 秒と数える、約 1/11)。
  - やり直し: cap 59.6 秒で 231.8 秒 (Local-MIP 205.6 秒を手間 10.8 秒、約 1/19)、期限を見ないので上限を 113 秒越えた。
- 直した版 (`fix_t1`): 600.50 秒 (5 ノード)。早めの探索 44.6 秒 (cap 60)、やり直し 61.7 秒 (cap 79.7)。Local-MIP の数えは実時間の 1/1.9。
- まとめ: cryptanalysis 系の 1 スレッドの超過は、大きいもの (obj14 の 107 秒) がやり直し FJ の期限の見落とし + 連鎖の手の数え漏れ、
  小さいもの (obj16 の 0.5〜8.8 秒) がダイビングの LP の DSE の重み・解き直し (4 節)。どちらも 3 節の直しで 600.5 秒に収まる。

## 5. 直した版の確認 (事実)

- 時間の上限: fhnw-binpack4-4 の 1 スレッド・600 秒は 600.00〜600.01 秒で time_limit (3 回、`fix_t1`・`fix2_t1`・`fix3_t1`)。
  cryptanalysiskb128n5obj16 の 1 スレッドは 600.45・600.86・600.53 秒。neos859080 の 1 スレッド 60・120 秒は 60.00〜120.01 秒。
- cargo test (依頼の形、`--release` と python のリンク): 419 通過・0 失敗 (最終のビルドで 2 回)。
- fuzz (`fuzz/`): `mip_fuzz_vs_highs.py 300` を既定・`ENOMOTO_MIP_THREADS=1`・`ENOMOTO_MIP_SYM_SST=1` で bad 0 / 300 (各)。
  `mip_fuzz_symmetric.py 300` を SST で bad 0 (SST の行が入ったのは 18 件だけ)、オービトープを切って SST に回す形
  (`ENOMOTO_MIP_NO_ORBITOPE=1`、規模 1・2) で bad 0 / 300 × 2 (対称な 199・222 件の全部で SST の行が入った)。
- 1 スレッドの既定の道は 2 の直しで変わる (早めの探索が早く終わる)。neos859080 の 1 スレッド 60 秒: ノード 3.9 万 → 13.6 万
  (早めの探索 46 秒 → 6.7 秒)。広い効き目は未計測 (6 節の nct_t1)。

## 6. SCIP は何で実行不能を示しているか (事実、120 秒・種 1 で 1 つずつ切った)

| 設定 | fhnw-binpack4-4 | neos859080 |
|---|---|---|
| 既定 | 10.5 秒 (8998 ノード) | 1.9 秒 (966) |
| `misc/usesymmetry=0` (対称性なし) | 時間切れ | 1.9 秒 (対称性なし) |
| `conflict/enable=False` | 58.4 秒 | 時間切れ |
| `presolving/maxrestarts=0` | 36.5 秒 | 2.0 秒 |
| `misc/usesymmetry=1` (対称性の制約) / `=2` (orbital fixing) / `=3` | 時間切れ | |
| `misc/usesymmetry=4` (Schreier-Sims の表のカットだけ) | 9.9 秒 | |
| `conflict/useprop=False` (伝播の衝突解析なし) | | 時間切れ |
| `conflict/useinflp=o` (LP の実行不能の衝突解析なし) / `=d` (双対の証明だけ) | | 時間切れ / 3.1 秒 |
| `presolving/maxrounds=0` / カットなし | | 1.6 秒 / 0.1 秒 (118 ノード) |

- **fhnw-binpack4-4: Schreier-Sims の表のカット (SST cuts)**。SCIP の前処理の `symmetry` が 45 行を足し (9 生成元、群の大きさ 10^6.56)、
  根で 0 から動かない下界のまま木 (約 9000 ノード、衝突解析あり) で示す。orbital fixing・対称性の制約だけでは解けない。
- **neos859080: 伝播の衝突解析** (+ LP の実行不能の衝突)。前処理・カット・対称性は要らない。衝突は一般整数の境界を含む
  (SCIP の bounddisjunction 制約、232 本が大域で使われ、平均 7.3 リテラル)。HiGHS 1.15.1 も 1.9 秒・1102 ノードで示す。

### 当方に足りないもの

- fhnw-binpack4-4: 当方は同じ 9 生成元・23 軌道を検出するが、orbital fixing (+ SYMDYN の LexRed) だけで、SST のカットがない (lex-leader の行は
  orbital fixing の既定で 0 本)。→ 7 節の試作。
- もう 1 つの浪費: 実行可能性問題では根の後のやり直し FJ (robust `g`) が**残りの 0.5** (秒の上限なし) を使い、その間は根が止まる。
  実行不能な実行可能性問題では丸ごと空回り (4 スレッド 120 秒で 53.7 秒、600 秒なら約 290 秒)。1 スレッドでは早めの探索 (2 節の数えのずれ) も重なり、
  木に入れずに終わる (直した版の 1 スレッド 600 秒は 0 ノード)。
- neos859080: 当方も衝突解析 (HiGHS 式、リテラルの衝突のプール) を持ち、6000 本ほど足す。池の寿命・大きさ (`ENOMOTO_T_MIP_LIT_CONFLICT_AGE`・
  `_POOL`) を広げても 60 秒で決着しない (ノード 5.5 万・5.3 万)。当方の結果は種で大きく揺れる (過去の記録で既定 4 スレッドが 1.6〜3.0 秒の回と
  48.6 秒・600 秒時間切れの回、cand1113・review2 など)。今のビルドの 1 スレッドは種 1〜4 とも 120 秒で決着しない。
  SCIP・HiGHS は安定して約 2 秒・1000 ノード。原因 (衝突の質か分枝か) はまだ切り分けていない。

## 7. 試作 `ENOMOTO_MIP_SYM_SST=1` (既定は不変)

- `symmetry::sst_pairs_except`: orbital fixing の代わりに Schreier-Sims の表のカット `x_l - x_k >= 0` を足す。各段で今の群が動かす最小の番号 `l` を
  leader にして、その軌道の全部の列 `k` に行を足し、`l` の固定部分群 (Schreier の補題の生成元、重複・恒等を除き `ENOMOTO_T_MIP_SYM_SST_GENS`
  (64) 本まで) に進む。手間 (`ENOMOTO_T_MIP_SYM_SST_WORK`、5e7) を越えたらそこまで。
- leader を「動かす最小の番号」にとるので、列の番号の辞書式で最大の解は全段のカットを満たす (既存の lex-leader の行・`SYM_LEX_GENS` での並べ替えと
  矛盾しない)。生成元を間引いた部分群でもカットは正しい (減るだけ)。オービトープの生成元と、オービトープの列を動かす生成元は除く (lex-leader と同じ)。
  サブ MIP では使わない。
- fhnw-binpack4-4: **45 行・9 段** (SCIP の 45 行と同じ数)。
  - 1 スレッド、早めの探索とやり直しを切る (`ENOMOTO_MIP_FEAS_FJ_EARLY=0 ENOMOTO_MIP_ROBUST=pq`): SST **36.9 秒・9757 ノードで infeasible**、
    SST なしは 300 秒・13.7 万ノードで時間切れ。
  - 4 スレッド・既定のまま + SST: 77.97 秒 (種 1)・77.06 秒 (種 2) で infeasible、SST なしは 120 秒で時間切れ (8.3 万ノード)。
    78 秒のうち 53.7 秒はやり直し FJ (6 節)。`ENOMOTO_T_MIP_ROBUST_FJ_FEAS_FRAC=0.05` を足すと 24.9 秒。
  - 1 スレッド・既定 + SST: 600 秒で 0 ノード (早めの探索 130 秒 + やり直し 470 秒)。`ROBUST_FJ_FEAS_FRAC=0.05` を足すと 242.6 秒で infeasible。

## 8. 対策の立案

1. (済) 期限の直し 3 つと連鎖の手の数え。
2. SST の既定化の判断: 対の比べ (9 節の MSST・PSST・X) で、対称性のある問題の得失と誤答を見る。orbital fixing と併せる形
   (SCIP の既定 7 = SST + orbital fixing。SST の後の固定部分群で orbital fixing を続ける) は次の段。
3. 実行可能性問題のやり直し FJ の上限: 残りの 0.5 は実行不能な問題で時間の半分を捨てる。秒・手間の上限を足す (例 `ROBUST_FJ_FEAS_FRAC` を 0.05〜0.1 に、
   または根の木の進み (下界が動かない) を見て打ち切る)。ns1116954 (この 0.5 の理由) を含めた対の比べが要る。
4. Local-MIP の手間の見積り: 固定の `step_work` をやめ、手の評価で実際に触った行・列の非零を数える (FJ は逆に多め)。手間の上限の方針
   (決定性) を保ったまま、実時間とのずれを 2 倍以内に。
5. neos859080: 当方の衝突 (長さの分布・一般整数のリテラルの扱い・使われた回数) を SCIP・HiGHS の数と並べる。HiGHS は同じ HiGHS 式なので、
   HiGHS のログ (1102 ノード) と当方の同じ種のログを並べて、最初に分かれる所 (根の LP の頂点・分枝の選び・衝突の数) を探す。
   種による揺れが大きいので、まず 10 種 × 120 秒で当方の解けた割合を測る。

## 9. 対の比べの台本 (親への依頼)

`cfg.py`: def・def_t1 (このコミットの既定)、sst・sst_t1 (`ENOMOTO_MIP_SYM_SST=1`)、nct_t1 (連鎖の手を数えない、直す前の 1 スレッドの早めの探索)。
名前が `_t1` で終われば 1 スレッド、ほかは 4 スレッド。

- `shards_MSST.json` (Mittelmann 48 問 × 種 1・2 × {def, sst}、600 秒、12 分担、各 1.4 時間): `python run.py MSST K` (K = 0〜11)、`python summ.py MSST def sst`
- `shards_PSST.json` (MIPLIB 58 問 × 種 1〜3 × {def, sst}、60 秒、4 分担、各 0.74 時間): `python run.py PSST K` (K = 0〜3)、`python summ.py PSST def sst`
- `shards_MNCT_T1.json`・`shards_PNCT_T1.json` (同じ問題 × {def_t1, nct_t1}): `python run.py MNCT_T1 K`・`python run.py PNCT_T1 K`、
  `python summ.py MNCT_T1 def_t1 nct_t1` (見積りは 4 スレッドの時間からなので短めに出る)
- `shards_X.json` (fhnw-binpack4-4・neos859080・cryptanalysiskb128n5obj16 × 種 1・2 × {def, sst, def_t1, sst_t1}、600 秒、4 分担):
  `python run.py X K` (K = 0〜3)、`python summ.py X def sst`・`python summ.py X def_t1 sst_t1`
- 準備は analysis/vsscip_20261117/README.md と同じ (`pip install maturin highspy==1.15.1 && pip install -e .`、このコミット以降でビルド)。

## 10. 実行可能性問題のやり直し FJ の上限の試作 (2026-11-18 追加)

- 試作 `ENOMOTO_T_MIP_ROBUST_FJ_FEAS_SECS` (既定は上限なし = 従来の残りの 0.5): 目的が全部 0 の問題の根の後のやり直し FJ の秒の上限。
- 効く問題: 目的が全部 0 なのは Mittelmann 186 問 + MIPLIB 58 問 (計 203 問) のうち 7 問 (MPS の目的の行の非零を数えた):
  cryptanalysiskb128n5obj14・obj16、fhnw-binpack4-4・4-48、neos-3004026-krka、ns1116954 (P と M の両方)、ns1952667。
- 手元 (事実): fhnw-binpack4-4 + SST の 4 スレッド 120 秒は、上限なし 78.0 秒 → `ENOMOTO_T_MIP_ROBUST_FJ_FEAS_FRAC=0.05` で 24.9 秒 (7 節)。
  ns1116954 の 1 スレッド 60 秒 (3 本同時) は、上限なし・30 秒・10 秒とも種 1〜3 で時間切れ (差は見えない)。
- 対の比べ (親への依頼): `shards_F.json` = 7 問 × 種 1〜3 × {def, rfs30, def_t1, rfs30_t1}、600 秒、6 分担 (84 回):
  `python run.py F K` (K = 0〜5)、`python summ.py F def rfs30`・`python summ.py F def_t1 rfs30_t1`。
  採用の目安: 解けた回が減らない (特に ns1116954・binpack4-48 など解のある問題で FJ が見つける回を失わない) こと。

## 11. neos859080 の切り分け (2026-11-18 追加、事実)

1. 前処理の差: HiGHS・SCIP は前処理で 160 列 → 120 列、非零 1280 → 1128。SCIP の統計では linear の双対の前処理が 40 列を集約
   (`b_a + b_b ≤ 1` の 0-1 列の対。b は費用 0 で、他は `y ≤ 10 b` にしか出ない → 等式にして `b_b = 1 - b_a`)、sparsify が非零 72 を打ち消す。
   当方の前処理は列を 1 本も減らさない (行 164 → 160)。
2. SCIP の前処理後の問題 (`trans=True` で書き出し) を当方で解くと、1 スレッド・種 1〜6 の全部で 7.7〜9.1 秒で infeasible。
   SCIP の前処理から sparsify だけ外した問題は種 1・2 とも 60 秒で時間切れ。
3. 試作 2 つ (どちらも既定では使わない):
   - `ENOMOTO_MIP_DUAL_TIGHT_INEQ=1` (`src/presolve/dualtightineq.rs`): 双対の議論で不等式行を等式に上げる (整数列でも成り立つ形。条件は
     ファイルの説明)。上げたら止まっていた doubleton をもう一度動かす。neos859080 で 40 本を上げ、列 160 → 120。
   - `ENOMOTO_MIP_SPARSIFY=1`: 未使用だった `src/presolve/sparsify.rs` (台が含まれる等式だけ、フィルインなし) を集約の前に呼ぶ。
   - 両方で当方の前処理後も 120 列・非零 1128 (HiGHS・SCIP と同じ大きさ)、構造も同じ (違いは 60 項の行の係数 1 と 2 だけ。係数を 1 に
     締めても同じ)。それでも 1 スレッド 120 秒・種 1〜4 で全部時間切れ (境界は 2 → 3〜4)。
4. **並びの影響**: SCIP の前処理後の問題の行・列を並べ替えて書き直すと、当方は 3 通り中 2 通りで 60 秒の時間切れ (残る 1 通りは 9.0 秒)。
   当方の前処理後の問題を並べ替えた 3 通りも全部時間切れ。2 の「解ける」は SCIP の書き出しの並びが当たっただけと見られる。
   一方、SCIP は `randomization/permutationseed` 1〜6 (行・列の並べ替え) で 0.5〜2.4 秒、HiGHS は元の問題の並べ替え 3 通りで 1.7〜3.2 秒。
5. 結論 (推測を含む): neos859080 の差は前処理ではなく、木の探索 (衝突解析・分枝) の頑健さ。当方は並び次第で数秒にも数百秒の時間切れにも
   なる裾の重い問題で、SCIP・HiGHS は並びによらず数秒。SCIP では伝播の衝突解析を切ると解けない (6 節)。
6. 次の手 (計画): 並びで解けた回 (SCIP の書き出しの並び、種 1) と解けない回 (並べ替え 1) を同じ問題でログに取り、衝突の数・長さ・
   大域で効いた数 (SCIP は 232 本・平均 7.3 リテラル)・分枝の選び・再スタートの有無を並べる。HiGHS は 4 回の再スタートで 120 → 69 列に
   縮めている (木で見つけた大域の固定を前処理にかけ直す) ので、当方の再スタートの条件 (`restart check: newly fixed 0`) も見る。
- 正しさ: 2 つの試作を両方入れた fuzz (`fuzz/`): mip_fuzz_vs_highs 300 (4 スレッド・1 スレッド)・mip_fuzz_symmetric 300 で bad 0。
  ログつきの mip_fuzz_vs_highs・fixed_charge・clique 各 300 で bad 0、等式化が動いた前処理 1244 回・sparsify が動いた前処理 535 回 (`fuzz/dti_sp_coverage.txt`)。
- cargo test: 422 通過 (dualtightineq の単体試験 3 件を含む)。

## 12. PNCT_T1 の分担の報告から (2026-11-18、事実)

- 分担 0・1・3 (ビルド fcce29cdb、各 86〜88 行): def_t1 と nct_t1 で optimal の顔ぶれ・目的値は一致、crash 0、時間の差は小さい
  (分担 3 は時間比 1.5 倍超の対なし)。時間切れの暫定解の差が数問 (neos-1456979 種 1 は nct_t1 が良い、10teams 種 1 は def_t1 が良い など)。
- 60 + 1 秒を越えた行: ns1116954 の **nct_t1** の種 2 (66.8 秒)・種 3 (68.2 秒)。0 ノード・根の LP の途中で止まった回。同じ種の def_t1 は 60.3・60.6 秒。
- 手元の追試: nct_t1 の種 2・3 は 56 秒前後で FJ が解を見つけて optimal (CPU を食う裏の処理 2 本を足しても同じ)。時間の上限を根の LP の途中
  (20・22 秒) に置くと、根の LP は期限の 0.35 秒後に止まる (「root LP stopped at the time limit (… 0.35s past the deadline)」、
  `logs/ns1116954_rootlp2*_t1.*`、行 12.8 万・列 1.2 万の LP)。期限の後は MipProblem の片付け (drop) にも 0.3 秒ほどかかる。
  計測機の 6.8〜8.2 秒の超過は再現できていない (推測: 計測機では早めの探索の後に根の LP が期限をまたぎ、こちらで見えない道 (根の LP の救済・
  尺度を変えた解き直しなど) を通った)。再現には計測機で `watch_at.sh ns1116954 1 60 61.5 名前 ENOMOTO_T_MIP_SEED=2 ENOMOTO_MIP_FEAS_EARLY_NO_CHAIN_TICKS=1`
  を回し、`logs/` のスタックとログを見る。nct_t1 は「直す前」の形なので、既定 (def_t1) では起きにくい。

## 13. highschool1-aigio の 4 スレッドの超過 (2026-11-18、親からの知らせ、事実)

- 知らせ: sparsify J 分担 7 (ビルド e9342a5fb、4 スレッド・600 秒) で def 605.9 秒・spls 604.2 秒 (0 ノード・下界 −∞)。
  e9342a5fb には HANG の直し (fc21f80d5・00585f12c) は入っている。
- 再現 (今のビルド、`logs/highschool1-aigio_hs_t4.*`): 606.06 秒。上限を 200 秒にしても 203.29 秒 (`hs200_t4`)。
  - ログ: 根の LP は「the time limit (160.51s, 0.24s past the deadline)」で止まる。LP 自身は自分の期限の 0.24 秒後に止まっているが、
    求解の時計 (solver の開始から) で 160.5 秒 + 前処理 42.7 秒 = 203.2 秒で、**LP に渡した期限が全体の期限より約 3 秒遅い**。
  - 原因: `solve_mip` が畳み込み (`fold::solve_folded_any`) の判定に 2.7〜2.9 秒使い (「MIP_FOLD: no fold (n 320404 m 92568) in 2.733s」)、
    畳まないと決めた後の求解 (`dispatch`) に時間の上限を**丸ごと**渡していた (前処理の時計 `t_pre` はその後から数える)。
    列 32 万の問題では畳み込みの判定 (と問題の組み立て) の分だけ必ず越える。
- 直し (既定): `solve_mip` の入口からの経過を、畳み込みの判定と `dispatch` に渡す時間の上限から引く (`ENOMOTO_MIP_NO_ENTRY_DEADLINE` で直す前)。
  - 確認: 200 秒の上限で **200.05 秒** (「solve returned TimeLimit after 199.97s」、根の LP は 0.02 秒遅れ、`hs200fix_t4`)。
  - cargo test 422 通過、mip_fuzz_vs_highs 300 で bad 0 (`fuzz/vs_highs_entry.txt`)。
- 期限の直後のスタック (`hs_t4.gdb`) では、根の LP の競争の内点法 + クロスオーバーの側 (`BasisSelector::try_add`) が期限の後も回り続けていた。
  主は待たない (競争の負けは置き去り) ので時間には効かないが、CPU を 1 本使い続ける。別の直しの候補 (未着手)。
- 推測: 大きい問題で 600 秒をわずかに越える回 (例えば 600.5〜6 秒) のうち、畳み込みの判定が数秒かかるものはこれで説明がつく。

### 13.1 負けた内点法 + クロスオーバーが期限の後も回る件 (群 5 の試作 `ENOMOTO_IPM_G5CANCEL` で扱う)

- 群 5 の `ENOMOTO_IPM_G5CANCEL` の初版 (33433b0a7) は正規方程式・拡大系の組み立ての中だけで止めの合図を見ていて、クロスオーバーの
  基底の選び直し (`BasisSelector::try_add` を列ごとに回すループ、`src/simplex/crossover.rs`) では見ていなかった。
- HANG でも同じ環境変数で毎列見る形を作って確かめた (e9c699f5e): highschool1-aigio・4 スレッド・200 秒で、期限 (200.0 秒) の直後に
  「basis selection stopped by the cancel signal after 98486 columns」、求解は 200.08 秒 (`logs/highschool1-aigio_hs200g5_t4_s1.log`)。
  G5CANCEL つきの mip_fuzz_vs_highs 300・lp_fuzz_vs_highs 300 で bad 0 (`fuzz/vs_highs_g5xo.txt`・`fuzz/lp_g5xo.txt`)。
- 同じ時期に群 5 が同じ場所に `g5cancel_tick` (256 列ごとに見る) を入れていたので、取り込みのときに群 5 の版を採った (重複を避ける)。
  この問題では 1 列の消去が平均 1.6 ミリ秒 (9.8 万列で 156 秒) なので、256 列ごとだと止まるまで平均 0.4 秒ほど遅れうる (推測)。
- 群 5 の版 (73caafdb9、256 列ごと) の確認 (先頭 38c11be09 = 9cf76445f と群 5 の直しの両方入り、highschool1-aigio・4 スレッド・200 秒):
  - 求解の時間: G5CANCEL=1 で 200.05・200.06 秒 (種 1・2)。根の LP は期限の 0.02〜0.03 秒後に止まる。超過は 0.1 秒未満。
  - 負けた側が止まるか: 求解の後にプロセスを 12 秒生かしておき、スレッドごとの CPU 時間を 1 秒ごとに見た (`g5check/`、手元だけの一時的な
    ログの 1 行で止まった時刻を出した。コミットはしていない)。G5CANCEL=1 は求解が戻ってから **0.62 秒後** に基底の選び直しが止まり
    (k = 99328 列)、内点法のスレッドは消えた。G5CANCEL=0 は求解が戻った後も 11 秒間、内点法のスレッド 2 本が CPU を使い続けた
    (それぞれ 1 秒に約 1 秒ずつ増える)。
- 群 5 の 16 列ごとの版 (d37a529f9) の確認 (先頭 715f000a1、同じ条件・同じ測り方、`g5check/*_16cols_*`): 求解 200.09・200.13 秒
  (根の LP は期限の 0.05・0.10 秒後に停止)。基底の選び直しは求解が戻る 0.11・0.19 秒**前** (期限の直後) に止まり (k = 98832・100320)、
  求解の後の 11 秒間に内点法のスレッドは残っていなかった。種 2 ではこのほかに、166 秒前 (前処理の LP プロービングの競争と見られる) にも
  k = 77456 で止まっている (二段解法が勝った後の負けた側が、合図で止まった回)。

## 14. F の集計 (実行可能性問題のやり直し FJ の秒の上限、2026-11-18、事実)

ビルド f3ea0afb3、7 問 × 種 1〜3 × {def, rfs30, def_t1, rfs30_t1}、600 秒、84 行 (分担 0〜5)。`summ_F_4t.txt`・`summ_F_t1.txt`。

| | def / rfs30 (4 スレッド) | def_t1 / rfs30_t1 (1 スレッド) |
|---|---|---|
| 解けた数 (21 組) | 9 / 9 | 9 / 9 |
| 食い違い・crash | 0・0 | 0・0 |
| SGM (どちらかが解けた 3 問) | rfs30 / def −9.3% [−24.7%, +13.8%] | rfs30_t1 / def_t1 −8.0% [−18.1%, +7.4%] |
| 最大 RSS | 1749 / 1896 MB | 822 / 822 MB |

- 解ける 3 問 (binpack4-48・krka・ns1116954) は全設定・全種で解け、上限 30 秒で解を失った回はない (懸念の ns1116954 も 3.2〜16.0 秒で全部解ける)。
  時間の差は種の揺れの範囲 (区間が 0 をまたぐ)。
- 決着しない問題では木が進む (ノード数、種 1・2・3):
  - fhnw-binpack4-4 の 1 スレッド: def_t1 は 3 種とも **0 ノード** (やり直し FJ が残りを使い切る) → rfs30_t1 は 11.3 万・18.4 万・7.6 万。
    4 スレッド: def 16.1 万・35.7 万・18.4 万 → rfs30 63.5 万・39.2 万・72.3 万。
  - ns1952667 の 4 スレッド: def 2.2 万・5.3 万・3.3 万 → rfs30 6.0 万・13.7 万・6.1 万 (1 スレッドは同程度)。
  - cryptanalysis obj14・obj16 は全設定で 0〜19 ノード (根の LP が重く、差なし)。
- 時間の上限: 全 84 行で result.time は 600.0〜600.3 秒 (直した後の版で、もとの 900 秒の kill のような超過はない)。
- 判断 (推測を含む): rfs30 は解を失わず、実行不能な実行可能性問題で木に時間を回す。binpack4-4 は SST (7 節) と組み合わせて初めて解ける形
  (SST + `ROBUST_FJ_FEAS_FRAC=0.05` の 4 スレッドで 24.9 秒) なので、既定にする候補は「rfs30 (または相当の上限) を既定にする」、
  SST は MSST・PSST の結果を待って判断。7 問・3 種なので、既定にするなら P・M の全体の対の比べ (目的が 0 でない問題には影響しないので回帰は
  起きない見込み) は省いてよい。

### 14.1 入口の期限の直し (9cf76445f) の確認 (F から)

- F の時間切れ 48 行の result.time は最大 600.3 秒 (直す前の vsscip では cryptanalysis 系の 1 スレッドが 608.8・706.9 秒)。wall は最大 602.6 秒で、
  wall − time (Python の起動と MPS の読み込み、計測の外) は最大 2.6 秒。求解の中の超過は 0.3 秒以内。
- 畳み込みの判定が数秒かかる大きい問題 (highschool1-aigio など) の確認は 13 節 (200 秒で 200.05 秒)。M の全体では MSST の def 側の時間切れの行で同じ見方ができる。

## 15. MSST・PSST の採否の段取り (SST = `ENOMOTO_MIP_SYM_SST=1`)

1. 揃ったら `python summ.py MSST def sst > summ_MSST.txt`・`python summ.py PSST def sst > summ_PSST.txt`。
2. 正しさ: 食い違い (最適値の不一致・最適と実行不能の衝突) 0、crash 0。1 つでもあれば採らず、その (問題, 種) を `one.sh` で切り分ける。
3. 対称性のある問題に絞った見方: SST が効くのは「orbital fixing の生成元があった問題」だけ (ほかは同じ道)。def のログの
   「MIP: symmetry: … generators」が 0 でない問題を、手元で `ENOMOTO_MIP_SYM_DETECT_ONLY=1 ENOMOTO_MIP_LOG=1` (検出だけで止める) で数え直して表に分ける
   (全体の SGM は対称性のない問題で 0 に薄まるため)。
4. 採用の目安: 解けた数が減らない、対称性のある問題の SGM の区間の上端が +5% を越えない、時間切れの終わりのギャップが悪くならない。
   orbital fixing を外すので、orbital fixing が効いていた問題 (例えば orbitope でない対称の問題) が遅くなる側の確認を特に見る。
5. 悪い問題があれば、SST と orbital fixing の併用 (SCIP の既定 7 と同じ形: SST の後の固定部分群で orbital fixing) を次の試作にする。
6. rfs30 (14 節) と SST の組み合わせは binpack4-4 で効く (4 スレッド 24.9 秒相当) ので、両方採るなら X (3 問 × {def, sst, def_t1, sst_t1})
   を rfs30 込みで取り直す。
