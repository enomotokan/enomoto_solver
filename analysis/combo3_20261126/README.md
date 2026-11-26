# c2c の新しい版 c2c v3 (`c3`) の組み立てと前測り (2026-11-26、親の依頼)

## 0. c3 の形 (cfg.py)

c3 = 今の c2c (analysis/combo2_20261125/cfg.py の C2C = C2 + SB_PROBEBD=1) に、次の 4 つの部品を足したもの。

| 部品 | 環境変数 | c2c の値 → c3 の値 | 出どころ |
|---|---|---|---|
| nrap | ENOMOTO_MIP_NODEROWS_ADAPT | r0k23 → r0k23p | analysis/nrashare_20261126 3 節 (neos-911970 の害を消す) |
| share | ENOMOTO_MIP_NRA_SHARE | (なし) → 1 | analysis/nrashare_20261126 6 節 (P の 4 スレッドで中立) |
| enumt | ENOMOTO_MIP_ENUM_ADAPT | 1s → 1st | analysis/nrashare_20261126 7 節 (主でないスレッドの前処理の時計の直し、P で確かめ中) |
| pskip | ENOMOTO_MIP_SB_PROP_PSKIP | (なし) → 1 | analysis/c2c1t_20261126 (伝播込みの強分枝の子の伸びを擬似費用に入れない、P で確かめ中) |

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

(結果は回し終えてから書く)

## 3. 第 2 段: MIPLIB 240 問の台本 (s2/、研磨 egj と強分枝の並列 PARSB の結果がそろってから回す)

台本は combo2 の第 2 段 (analysis/combo2_20261125/s2) の写しで、設定の名前を c2c → c3 に変えただけ。分担の組み替え (takeover.json・`--tail`)・
OOM 2 回で確定する決まりもそのまま。

- 設定: def_t1・def_t4・c3_t1・c3_t4 (s2/cfg.py。c3 は ../cfg.py の C3)。種 1・上限 600 秒。
- 問題: s2/problems.json (combo2 の第 2 段と同じ 240 問: M 186 問 + 残り 54 問)。s2/solu.json も同じ。
- 分担: `python analysis/combo3_20261126/s2/plan.py` → s2/shards.json (11 分担、960 回、見積りは各 10.7 時間)。
  同じ (問題, スレッド) の def と c3 は、同じ分担で続けて回す。誰がどの分担を回すかは親が決める。
- 版: 回す前に親が決めて、ここに書く (第 1 段の版から進んでいれば、check_default と c3 の fuzz を回し直す)。

### 手伝いの子が回す手順 (分担 K)

1. ブランチ ccr-1beed81b-ex0uhq を取り込み、決めた版のソースで `pip install -e .`
   (先端が決めた版と違えば、`git checkout <版> -- src python Cargo.toml Cargo.lock pyproject.toml` してからビルドし、
   `git checkout HEAD -- src python Cargo.toml Cargo.lock pyproject.toml` で作業ツリーを戻す)。
2. `python -c "import highspy"`。なければ `pip install highspy==1.15.1`。
3. `python analysis/protoscreen_20261124/check_default.py` が「全部一致」になることを確かめる。
4. `python analysis/combo3_20261126/s2/run.py K` → `analysis/combo3_20261126/s2/res_sK.jsonl`。
   - 止めても同じコマンドで続きから回る。2 時間の上限で止まったら、同じコマンドで回し直す。
   - 計測の途中でビルドし直さない。src・python/ の変わる取り込みもしない。setsid・nohup は使わない。
5. 進みは `python analysis/combo3_20261126/s2/prog.py`。
6. 結果の行は区切りごとに commit し、push の前に `git fetch origin ccr-1beed81b-ex0uhq && git merge --no-edit FETCH_HEAD`。

集計: `python analysis/combo3_20261126/s2/summ.py > analysis/combo3_20261126/s2/summ.txt`
(スレッド別 × 集合別 (M・240 問全部・残り)。../kind.json の「道筋」「揺れ」の問題は遅れに数えない。kind.json は combo2 のものの写し)。

## 1.1 c3 の誤答の確かめ (`fuzz_c3.out`、188008a5d)

- fuzz 7 本: 誤答 0 (fuzz_tiny 0/400・0/400、fuzz_highs 0/300、fuzz_objint 0/1000・0/1000、fuzz_slack mismatches 0 (1000・1000 回))。
- cargo test: 424 通過・**1 失敗**。失敗は `mip::alns_cp::tests::adaptive_moves_like_cpsat` (src/mip/alns_cp.rs:244 の
  `(c.value - 0.5).abs() < 0.1`)。ほかの子が足した試作 ALNS (a6beb833d) の単体の試験で、環境変数を読まない (`Adaptive` の増減だけを試す) ので、
  c3 とは関係がない。既定でも同じく失敗する見込み (計測中で作り直せないので、まだ確かめていない)。
- 注: 計測中に、この試験を確かめるつもりで cargo test を走らせ、約 2 分ビルドが重なった (すぐ止めた)。重なったのは分担 0 の
  mzzv42z の c3_t4・def_t1 (種 1、どちらも 60 秒の時間切れ)。この 2 行は消して回し直す。
