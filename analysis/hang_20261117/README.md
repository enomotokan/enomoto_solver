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
