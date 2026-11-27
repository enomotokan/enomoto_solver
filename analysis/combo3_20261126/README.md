# c2c の新しい版 c2c v3 (`c3`) の組み立てと前測り (2026-11-26、親の依頼)

## 0. c3 の形 (cfg.py)

c3 = 今の c2c (analysis/combo2_20261125/cfg.py の C2C = C2 + SB_PROBEBD=1) に、次の 4 つの部品を足したもの。

**2026-11-26 夜の差し替え (親の判断)**: pskip を外し、nolp (`ENOMOTO_MIP_SB_PROP_NO_LP=1`) を入れた。今の c3 = c2c + nrap + share + enumt + nolp。
根拠は analysis/c2c1t_20261126/README.md 10 節 (P で c2c 比 1 スレッド 1.012・4 スレッド 1.003、binkar10_1・nu25-pr12 の害なし、eil33-2 は伝播つきの子の LP の得を失う分 1.16〜1.23 倍) と、この README 2 節 (pskip の仕組みの害)。2 節の前測りの結果は pskip 入りの形のもの。

| 部品 | 環境変数 | c2c の値 → c3 の値 | 出どころ |
|---|---|---|---|
| nrap | ENOMOTO_MIP_NODEROWS_ADAPT | r0k23 → r0k23p | analysis/nrashare_20261126 3 節 (neos-911970 の害を消す) |
| share | ENOMOTO_MIP_NRA_SHARE | (なし) → 1 | analysis/nrashare_20261126 6 節 (P の 4 スレッドで中立) |
| enumt | ENOMOTO_MIP_ENUM_ADAPT | 1s → 1st | analysis/nrashare_20261126 7 節 (主でないスレッドの前処理の時計の直し、P で確かめ中) |
| ~~pskip~~ (外した) | ENOMOTO_MIP_SB_PROP_PSKIP | (なし) → 1 | analysis/c2c1t_20261126 (伝播込みの強分枝の子の伸びを擬似費用に入れない) |
| nolp (pskip の代わり) | ENOMOTO_MIP_SB_PROP_NO_LP | (なし) → 1 | analysis/c2c1t_20261126 10 節 (伝播の境界を子の LP に渡さない。判定は値の有無だけで、0 でも有効) |

- 環境変数の判定はコードで確かめた (先端 188008a5d):
  - `nra_share_on`・`sb_prop_pskip` は 0 でなければ有効。
  - ENUM_ADAPT の `t` は、src/mip/mod.rs `EnumClockGuard::new` が値に `t` を含むかで見る。
- 部品の切り替え: cfg.py の `USE` から名前を消すと、c3 からその部品が外れる (第 2 段の s2/cfg.py も同じ C3 を読む)。
  外した形を別に作るなら `c3_without("enumt", "pskip")` のように書く。
- `python analysis/combo3_20261126/cfg.py` で c3 の環境変数の一覧を出す。

## 1. 版

- 先端 188008a5d で `pip install -e .`。既定に PAR4=fmr・ROOTPAR_CUT・PARIDLE・UNIFY_FIX が入った版。
- check_default: 6 問とも基準と一致。
- c3 の誤答の確かめ (fuzz 7 本と cargo test、`fuzz_c3.out`): 2 節。

## 2. 第 1 段: P の前測り (1・4 スレッド、種 1〜3、60 秒)

- 設定: def_t1・def_t4・c3_t1・c3_t4 (cfg.py の CFGS)。58 問 × 種 3 × 4 設定 = 696 回。
- 分担: `plan.py 2` → shards.json (2 分担、各 3.2 時間の見積り)。`python analysis/combo3_20261126/run.py K` → `res_sK.jsonl`。
- 集計: `python analysis/combo3_20261126/summ.py > summ.txt` (combo2 の第 1 段の summ.py の写し。c3_tN を def_tN と比べる)。

- 版: 188008a5d (分担 0 は私、分担 1 は FinV が同じ版の worktree で回した)。696 回、crash 0。
  分担 0 で cargo test のビルドと約 2 分重なった mzzv42z の 2 行 (1.1 節) は消して回し直した。

| スレッド | c3 / 既定 | 95% 区間 | 解けた数 既定 → c3 | 全種 1.25 倍超の遅れ (仕組み・未分類) | 同 (道筋・揺れ) | 食い違い |
|---|---:|---|---|---|---|---|
| 1 | 0.984 | [0.942, 1.028] | 127 → 126 | なし | qiu・neos-960392 | 0 |
| 4 | 1.009 | [0.984, 1.034] | 138 → 139 | roll3000・binkar10_1・nu25-pr12 | neos-960392 | 0 |

(参考: combo2 の第 1 段の c2c は、1 スレッド 0.962 [0.906, 1.021]・4 スレッド 0.996 [0.976, 1.017]。版が違うので直接は比べられない)

- 1 スレッドで解けた数が 1 減ったのは、時間切れと解けたの入れ替わり (10teams・pk1 が解け、mc11・roll3000 の種 1 つずつが 60 秒に届かない) の差 1。
- 4 スレッドの未分類の遅れ 3 問を、部品を 1 つずつ外して分けた (`abl.py` → `res_abl.jsonl`、4 スレッド・種 1〜3、60 秒、版 188008a5d)。

| 問題 | def | c2c | c3 | c3−nrap | c3−share | c3−enumt | c3−pskip |
|---|---|---|---|---|---|---|---|
| nu25-pr12 | 3.49 / 3.41 / 3.34 | 3.36 / 4.01 / 3.11 | 4.70 / 4.19 / 4.58 | 4.39 / 4.03 / 4.63 | 4.57 / 4.04 / 4.27 | 4.68 / 4.24 / 4.64 | 3.13 / 4.16 / 3.24 |
| binkar10_1 | 3.91 / 4.09 / 4.42 | 4.01 / 3.78 / 3.74 | 5.44 / 6.39 / 5.29 | 5.12 / 5.15 / 5.41 | 5.33 / 5.19 / 5.80 | 5.41 / 5.63 / 5.29 | 3.70 / 4.20 / 4.09 |
| roll3000 | 26.6 / 27.5 / 39.0 | 25.9 / 30.8 / 25.2 | 36.6 / 37.8 / 26.3 | | | | 21.8 / 28.8 / 31.7 |

- **nu25-pr12・binkar10_1 の遅れは pskip (SB_PROP_PSKIP)**。pskip を外すと既定・c2c 並みに戻り、ほかの 3 部品は外しても変わらない。
  - binkar10_1 (種 1、ログつき、主のスレッドの数): c3 は分枝の選び 479 回・強分枝の LP 1507 回・ノード 3458、pskip なしは 323 回・925 回・2394。
    伝播の境界を子の LP に渡した強分枝の子は 8858 (pskip なしでも 7531) で、SB_PROBEBD の下では強分枝の子のほとんどが伝播つきになる。
    pskip はその全部の観測を擬似費用に入れないので、擬似費用が育たず、ノードと強分枝が 1.4〜1.6 倍に増える (仕組みの害)。
  - 1 回が 3〜4 秒の小さい問題で 1〜2 秒の差なので、ずらした比では目立たないが、全種で同じ向き。
- roll3000 は pskip を外すと 種 1・2 は速く、種 3 は遅い (幾何平均 c3 33.1、c3−pskip 27.1、既定 30.7)。揺れが大きく、pskip の害とまでは言えない。
- enumt・share・nrap は、この 3 問では害がない。

## 3. 第 2 段: MIPLIB 240 問の台本 (s2/、研磨 egj と強分枝の並列 PARSB の結果がそろってから回す)

台本は combo2 の第 2 段 (analysis/combo2_20261125/s2) の写しで、設定の名前を c2c → c3 に変えただけ。分担の組み替え (takeover.json・`--tail`)・
OOM 2 回で確定する決まりもそのまま。

- 設定: def_t1・def_t4・c3_t1・c3_t4 (s2/cfg.py。c3 は ../cfg.py の C3)。種 1・上限 600 秒。
- 問題: s2/problems.json (combo2 の第 2 段と同じ 240 問: M 186 問 + 残り 54 問)。s2/solu.json も同じ。
- 分担: `python analysis/combo3_20261126/s2/plan.py` → s2/shards.json (11 分担、960 回、見積りは各 10.7 時間)。
  同じ (問題, スレッド) の def と c3 は、同じ分担で続けて回す。誰がどの分担を回すかは親が決める。
- 版: **(空欄。PATHH_SLACK (Gm3w) の既定化が push された後の先端にする。親が知らせる)**。
  (0afbb53b9 で一度確かめた: 4 節。版を決め直したら、その版でもう一度確かめる)
  決めた版で check_default と、今の c3 (nolp 入り) の fuzz 7 本・cargo test を回し直してから始める。

### 手伝いの子が回す手順 (分担 K)

ほかの仕事と同じ入れ物で回すときは、計測専用の worktree を作る (`pip install -e .` はその入れ物の enomoto_solver をその worktree に向けるので、
同じ入れ物で別の計測やビルドを同時にしない)。`<版>` は 3 節の冒頭の版。

```bash
# 1. worktree を作り、決めた版のソースにする
cd /home/user/enomoto_solver
git fetch origin ccr-1beed81b-ex0uhq
git worktree add --detach /home/user/es_c3s2 FETCH_HEAD
cd /home/user/es_c3s2
git diff --stat <版> HEAD -- src python Cargo.toml Cargo.lock pyproject.toml   # 空なら次へ。空でなければ:
#   git checkout <版> -- src python Cargo.toml Cargo.lock pyproject.toml
ln -s /home/user/enomoto_solver/.miplib_cache .miplib_cache   # 問題のファイルを使い回す (なければ自動で取りに行く)
# 2. ビルドと確かめ
pip install -e .
python -c "import highspy" || pip install highspy==1.15.1
python analysis/protoscreen_20261124/check_default.py          # 「全部一致」になること
# 3. 計測 (2 時間の上限で止まったら同じコマンドで回し直す。setsid・nohup は使わない)
python analysis/combo3_20261126/s2/run.py K                   # → analysis/combo3_20261126/s2/res_sK.jsonl (worktree の中)
python analysis/combo3_20261126/s2/prog.py                    # 進み
```

- 計測の途中でビルドし直さない。worktree の src・python/ を変える取り込みもしない。
- 結果の取り込み (区切りごと): 本体のチェックアウトに写して commit し、取り込んでから push する。
  ```bash
  cd /home/user/enomoto_solver
  cp /home/user/es_c3s2/analysis/combo3_20261126/s2/res_sK.jsonl analysis/combo3_20261126/s2/
  git add analysis/combo3_20261126/s2/res_sK.jsonl && git commit -m "combo3 s2: 分担 K の途中の結果"
  git fetch origin ccr-1beed81b-ex0uhq && git merge --no-edit FETCH_HEAD && git push -u origin ccr-1beed81b-ex0uhq
  ```
- 入れ物を計測だけに使うなら、worktree を作らず本体のチェックアウトで 1〜3 を回してもよい (そのときは 1 の `git checkout <版> -- ...` の後、
  push の前に `git checkout HEAD -- src python Cargo.toml Cargo.lock pyproject.toml` で作業ツリーを戻す。.so は作り直さない)。
- 行の `cpu` 欄に、回した機械の CPU の型とメモリが入る (前回との比べの注記に使う)。

集計: `python analysis/combo3_20261126/s2/summ.py > analysis/combo3_20261126/s2/summ.txt`
(スレッド別 × 集合別 (M・240 問全部・残り)。../kind.json の「道筋」「揺れ」の問題は遅れに数えない。kind.json は combo2 のものの写し)。

前回 (combo2 の第 2 段、版 c8e468e25) との比べ: `python analysis/combo3_20261126/s2/vsold.py > analysis/combo3_20261126/s2/vsold.txt`。
今の既定 / 前回の既定、今の c3 / 前回の既定、今の c3 / 前回の c2c を、スレッド別・集合別に出す。**前回と今回は回した入れ物 (機械) が違う**
(前回の行には CPU の欄がなく、同じ型かは確かめられない。どちらも 4 コア・メモリ 15 GB)。版と機械の両方の差が混じった比として読む。

## 1.1 c3 の誤答の確かめ (`fuzz_c3.out`、188008a5d)

- fuzz 7 本: 誤答 0 (fuzz_tiny 0/400・0/400、fuzz_highs 0/300、fuzz_objint 0/1000・0/1000、fuzz_slack mismatches 0 (1000・1000 回))。
- cargo test: 424 通過・**1 失敗**。失敗は `mip::alns_cp::tests::adaptive_moves_like_cpsat` (src/mip/alns_cp.rs:244 の
  `(c.value - 0.5).abs() < 0.1`)。ほかの子が足した試作 ALNS (a6beb833d) の単体の試験で、環境変数を読まない (`Adaptive` の増減だけを試す) ので、
  c3 とは関係がない。既定でも同じく失敗する見込み (計測中で作り直せないので、まだ確かめていない)。
- 注: 計測中に、この試験を確かめるつもりで cargo test を走らせ、約 2 分ビルドが重なった (すぐ止めた)。重なったのは分担 0 の
  mzzv42z の c3_t4・def_t1 (種 1、どちらも 60 秒の時間切れ)。この 2 行は消して回し直す。

## 4. 版 0afbb53b9 での確かめ (2026-11-27、版を PATHH_SLACK の後に決め直す前の分)

- 0afbb53b9 は 81ad145f0 (RETRY_ZMAX)・3f7265069 (qiu の守り)・472c0e8cb (PARSB) を含む (`git merge-base --is-ancestor` で確かめた)。
- check_default (`s2/check_default.out`): 6 問とも新しい基準 (egj 後、mik-250-20-75-4 は 6277 ノード) と一致。ほかの子の試作は既定オフのまま。
- c3 (nolp 入り) の fuzz 7 本と cargo test (`s2/fuzz_c3.out`): 誤答 0。cargo test は 425 通過・0 失敗
  (1.1 節の ALNS の試験の失敗は、この版では直っている)。
- 版を決め直したら、その版で同じ確かめを回し直す。
