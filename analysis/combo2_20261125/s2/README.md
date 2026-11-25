# 組み直した組み合わせ c2c の第 2 段: MIPLIB 240 問 (2026-11-25、親の依頼)

第 1 段 (../README.md、P) の途中の結果から、親の判断で c2c を第一候補にした。
- c2c = c2 + SB_PROBEBD=1。中身は ../cfg.py の C2C で、この cfg.py はそれを読む。
- MIPLIB 240 問 (Mittelmann 186 問を含む) で、既定と c2c を同じ版で比べる。

台本は前回の combo (analysis/combo_20261124) の写しで、設定の名前だけ combo を c2c に変えた。分担の組み替え (takeover.json・`--tail`) と、OOM 2 回で確定する決まりもそのまま。

## 1. 設定と分担

- 設定: def_t1・def_t4・c2c_t1・c2c_t4 (cfg.py)。種 1・上限 600 秒。
- 問題: problems.json (前回と同じ 240 問: M 186 問 + 残り 54 問)。solu.json も前回と同じ。
- 分担: plan.py → shards.json。
  - 11 分担、960 回、見積りは各 10.7 時間 (前回と同じ LPT の分け方)。
  - 同じ (問題, スレッド) の def と c2c は、同じ分担で続けて回す。
- 誰がどの分担を回すかは、親が決める。

## 2. 版の固定 (親の指示、2026-11-25)

**版は c8e468e25 に固定する。** 既定と c2c は、この版のソースで作った .so で回す。
- 第 1 段の版 (a5008f9d8) との違いは、src/mip/solver.rs の試作 `ENOMOTO_MIP_POOL_CLOSE` (既定オフ) だけ。既定と c2c の道筋は変わらないはず。
- 確かめ (c8e468e25 で `pip install -e .`):
  - check_default: 全部一致。
  - c2c の fuzz 7 本と cargo test: (結果をここに書く)
- 先端がこの後に進んでも、計測のビルドは c8e468e25 のソースで作る。
  - 先端の src・python/ が c8e468e25 と同じなら (`git diff --stat c8e468e25 HEAD -- src python Cargo.toml Cargo.lock pyproject.toml` が空なら)、そのまま `pip install -e .` でよい。
  - 違えば、`git checkout c8e468e25 -- src python Cargo.toml Cargo.lock pyproject.toml` で固定の版のソースにしてから `pip install -e .`。そのあと `git checkout HEAD -- src python Cargo.toml Cargo.lock pyproject.toml` で作業ツリーを戻す (.so は作り直さない)。
- 行の `commit` 欄には回したときの HEAD が入るが、ビルドは c8e468e25 で同じ。

## 3. 回し方 (各分担)

1. ブランチを取り込み、2 節のとおり c8e468e25 のソースで `pip install -e .`。
2. highspy があるかを確かめる (`python -c "import highspy"`)。なければ `pip install highspy==1.15.1`。
3. `python analysis/protoscreen_20261124/check_default.py` が「全部一致」になることを確かめる (基準は ref_default.json)。
4. `python analysis/combo2_20261125/s2/run.py K` → `res_sK.jsonl`。
   - 止めても同じコマンドで続きから回る。
   - 2 時間の上限で止まったら、同じコマンドで回し直す。
   - 計測の途中でビルドし直さない。python/ の変わる取り込みもしない。
5. 進みは `python analysis/combo2_20261125/s2/prog.py`。組み替えは前回の README 7 節と同じ手順。
6. push の前に取り込む。

c2c の誤答の確かめ (fuzz 7 本・cargo test) は、第 1 段の 3 節で済んでいる (誤答 0、424 通過)。版が変わったら回し直す。

## 4. 集計 (summ.py)

`python analysis/combo2_20261125/s2/summ.py > summ.txt`。前回の combo の summ.py と同じ表で、スレッド別 (1・4) × 集合別 (M・240 問全部・残り) に出す。

前回からの変更 (利用者の方針、第 1 段の README 7 節):
- 1.25 倍超の遅れのうち、../kind.json で「道筋」「揺れ」の問題は、採用の目安の「遅れ」に数えない。表には「遅れ (うち仕組み・未分類)」の 2 つの数で出す。
- 一覧には、分け方 `[道筋]` などと、前回の原因 (analysis/combo_20261124/cause.json) を付ける。
