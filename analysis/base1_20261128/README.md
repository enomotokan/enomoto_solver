# c2c の土台の部品を 1 つずつ既定に足して測る (2026-11-28、親の依頼・利用者の承認)

analysis/combo3_20261126 8.4 節: c3 の新しい部品は、どれも c2c の土台の部品を変える形で、単独では働かない。部品ごとに既定化を決めるため、
土台の部品を 1 つずつ今の既定に足して、P (MIPLIB の 58 問) の 1・4 スレッド、種 1〜3、60 秒で既定と比べる。

## 1. 設定 (cfg.py)

| 名前 | 足す環境変数 | 備考 |
|---|---|---|
| heur | HEUR_ADAPT=1mkr | `r` は漏れの直し (analysis/heurfix_20261128) |
| heurfp | HEUR_ADAPT=1mkr + FP_STAGE3_LS=a | FP_STAGE3_LS は HEUR_ADAPT が開く FP の第 3 段にだけ効くので、単独では測らない |
| nra | NODEROWS_ADAPT=r0k23p + NRA_SHARE=1 | share は 4 スレッドだけ働く |
| enum | ENUM_ADAPT=1st | |
| sbpb | SB_PROBEBD=1 + SB_PROP_NO_LP=1 | c3 の nolp の形 |

既定 (def_t1・def_t4) と合わせて 12 設定 × 58 問 × 種 3 = 2088 回。`plan.py 8` → shards.json (8 分担、各約 2.4 時間の見積り)。

## 2. 版

- **3f3054603** (先端: 7d43fcdac の DUALEQ・RSTLOOSE・PRR_TIGHT の既定化と、HEUR_ADAPT の `r` を含む)。
- check_default (`check_default.out`) と、全部の部品を入れた形の fuzz 7 本・cargo test (`fuzz_all.out`): 3 節。

## 3. 確かめ (版 3f3054603)

- check_default (`check_default.out`): 6 問とも基準と一致 (mik-250-20-75-4 は 3287 ノード)。HEUR_ADAPT の `r` を足しても既定の道筋は変わらない。
- 全部の部品 (HEUR_ADAPT=1mkr・FP_STAGE3_LS=a・NODEROWS_ADAPT=r0k23p・NRA_SHARE=1・ENUM_ADAPT=1st・SB_PROBEBD=1・SB_PROP_NO_LP=1) を
  入れた形の fuzz 7 本 (`fuzz_all.out`): 誤答 0 (fuzz_tiny 0/400・0/400、fuzz_highs 0/300、fuzz_objint 0/1000・0/1000、fuzz_slack mismatches 0 (1000・1000 回))。
  cargo test は 428 通過・0 失敗。
- 割り当て: 0 は私、1 EwwjY、2 FcsJ、3 GZRx、4 FVbj、5 FinV、6 GU3WAp、7 FKBv (親の許しで直接頼んだ。4・5 は最初 GcYE・GGo5 に頼んだが、
  ほかの計測と機械が重なるので親の判断で付け替えた)。EwwjY・GZRx は今の計測 (PRESIZE240) の後、07:40 UTC ごろから。

## 4. 回し方 (分担 K)

ほかの仕事と同じ入れ物で回すときは、計測専用の worktree を作る (`pip install -e .` は入れ物の enomoto_solver をその worktree に向けるので、
同じ入れ物で別の計測やビルドを同時にしない)。

```bash
cd /home/user/enomoto_solver
git fetch origin ccr-1beed81b-ex0uhq
git worktree add --detach /home/user/es_base1 FETCH_HEAD
cd /home/user/es_base1
git diff --stat 3f3054603 HEAD -- src python Cargo.toml Cargo.lock pyproject.toml   # 空なら次へ。空でなければ:
#   git checkout 3f3054603 -- src python Cargo.toml Cargo.lock pyproject.toml
ln -s /home/user/enomoto_solver/.miplib_cache .miplib_cache
pip install -e .
python -c "import highspy" || pip install highspy==1.15.1
python analysis/protoscreen_20261124/check_default.py          # 「全部一致」(mik-250-20-75-4 は 3287 ノード)
python analysis/base1_20261128/run.py K                        # → analysis/base1_20261128/res_sK.jsonl (worktree の中)
```

- 2 時間の上限で止まったら、同じコマンドで回し直す。setsid・nohup は使わない。計測の途中でビルドし直さない。
- 結果は区切りごとに本体のチェックアウトに写して commit し、取り込んでから push する:
  ```bash
  cd /home/user/enomoto_solver
  cp /home/user/es_base1/analysis/base1_20261128/res_sK.jsonl analysis/base1_20261128/
  git add analysis/base1_20261128/res_sK.jsonl && git commit -m "base1: 分担 K の途中の結果"
  git fetch origin ccr-1beed81b-ex0uhq && git merge --no-edit FETCH_HEAD && git push -u origin ccr-1beed81b-ex0uhq
  ```

## 5. 集計

`python analysis/base1_20261128/summ.py > analysis/base1_20261128/summ.txt` (combo3 の第 1 段の summ.py と同じ表。部品ごとに、同じスレッドの数の既定と比べる)。
既定化の候補の目安: 区間の上端 ≲ 1.05、解けた数が減らない、全種で 1.25 倍を超える仕組みの害がない、食い違い 0。

## 6. 結果 (2026-11-28、2088 / 2088 回、`summ.txt`)

8 分担とも版 3f3054603 (各分担の報告: src などを 3f3054603 と同じにしてビルド、check_default 全部一致)。crash・取得失敗 0、**食い違い 0**。
比は部品 / 既定 (同じスレッドの数、ずらし 10 秒、上限 60 秒、58 問 × 種 3)。

| 部品 | 1 スレッド 比 [95% 区間] | 解けた数 | 4 スレッド 比 [95% 区間] | 解けた数 | 全種 1.25 倍超の遅れ (仕組み・未分類) |
|---|---|---|---|---|---|
| heur | 1.016 [1.000, 1.032] | 126 → 122 | 1.004 [0.984, 1.024] | 141 → 139 | なし (t4 で physiciansched6-2 1.27 は全種ではない) |
| heurfp | 1.011 [0.995, 1.026] | 126 → 123 | 1.004 [0.984, 1.025] | 141 → 141 | なし |
| nra | 1.006 [0.994, 1.018] | 126 → 125 | 0.995 [0.984, 1.007] | 141 → 140 | t4: gr4x6 (0.03 → 0.05 秒) |
| enum | 1.008 [0.998, 1.017] | 126 → 126 | 0.998 [0.983, 1.013] | 141 → 141 | なし |
| sbpb | 1.008 [0.997, 1.020] | 126 → 127 | 1.024 [1.004, 1.045] | 141 → 141 | t4: nu25-pr12・eil33-2・p0201 |

**解けた数の増減は、ほとんどが 60 秒の上限すれすれの回の入れ替わり**:
- 失った回: mc11 (既定 50・57 秒)・mzzv42z 種 3 (既定 56 秒、heur・heurfp・nra・enum の全部で失う = 既定の回が運よく間に合った)・pk1 種 2 (51 秒)・
  timtab1 種 1 (43 秒)・physiciansched6-2 t4 種 1 (19 秒、heur だけ)。
- 得た回: timtab1 種 2・3 (heurfp・sbpb・nra・enum)・roll3000 種 2 (enum)。

**全種の遅れの中身**:
- nra t4 の gr4x6 は 0.03 秒 → 0.05 秒で、素の時間の比が 1.25 を超えただけ (固定費の揺れ)。害とは数えない。
- sbpb t4 の 3 問は、どれも種 3 つで同じ向き: nu25-pr12 2.9 → 3.8 秒、eil33-2 13〜19 → 17〜25 秒、p0201 0.6 → 0.9〜1.2 秒。
  強分枝の子の伝播 (と NO_LP で子の LP に境界を渡さない分) の手間で、4 スレッドで小さな仕組みの遅れがある。区間の下端も 1 を超える (1.004)。

## 7. 既定化の候補 (部品ごと)

利用者の方針 (中立なら入れる、新しい閾値なし、仕組みの 1.25 倍超の遅れなし) で:

1. **enum (ENUM_ADAPT=1st)**: 候補。1・4 スレッドとも中立 (1.008・0.998)、解けた数も同じ、遅れなし。
2. **nra (NODEROWS_ADAPT=r0k23p + NRA_SHARE=1)**: 候補 (中立)。1.006・0.995、解けた数の −1 は上限すれすれの入れ替わり (mzzv42z 種 3・mc11 種 3)、
   gr4x6 は固定費の揺れ。
3. **heurfp (HEUR_ADAPT=1mkr + FP_STAGE3_LS=a)**: 4 スレッドは候補 (1.004、解けた数同じ)。1 スレッドは 1.011・解けた数 −3 (mc11 の 2 種・mzzv42z・pk1、
   どれも 50〜57 秒の上限すれすれ)。中立に近いが、1 スレッドの mc11 (問題単位 1.07)・pk1 (1.05) は小さく遅い向き。条件付き (600 秒の計測で確かめたい)。
4. **heur (HEUR_ADAPT=1mkr のみ)**: heurfp より悪い (1 スレッド −4・4 スレッド −2)。heurfp の形で入れるなら、こちらは要らない。
5. **sbpb (SB_PROBEBD=1 + SB_PROP_NO_LP=1)**: 1 スレッドは候補 (1.008、解けた数 +1) だが、4 スレッドは 1.024 [1.004, 1.045] で小さな仕組みの遅れが 3 問。
   既定化は見送り (4 スレッドの手間の直しが先)。
