# 自己再発火ループのルーチン・プロンプト(正本)

このループは Claude Code の Routine(スケジュール実行)
`netlib-bottleneck-improvement-loop`(`trig_01L4ZSDpUPiziNzCjC1tM8pR`)が
起動する。そのプロンプトは Routine 側に保存されており、リポジトリの
コードとは別管理になっている。

Routine は HTTP API 経由で作成されているため **エージェントからは
プロンプトを更新できない**(`update_trigger` が
`routine was created via "http_api"` で拒否される)。更新は人間が
<https://claude.ai/code/routines/trig_01L4ZSDpUPiziNzCjC1tM8pR> で行う必要がある。

そのため、**あるべきプロンプトの正本をここに置く**。Routine 側の
プロンプトを変更したいときは、まずこのファイルを更新し、その内容を
上記 URL に貼り付けること。両者が食い違っている場合、実際に動くのは
Routine 側であることに注意。

- 選定ロジックの実体は `scripts/select_problem.py`(テスト:
  `tests/test_select_problem.py`)にあり、プロンプトはそれを呼ぶだけである。
  したがって選定基準の変更はリポジトリ側だけで完結する。
- 一方、問題数(93)やステップ構成の変更はプロンプト側の更新が必要になる。

## 現在 Routine に設定されているプロンプトとの差分(未反映)

以下の3点が Routine 側に**まだ反映されていない**(2026-09-21 時点):

1. ステップ1の選定基準が「実行時間を第一基準、同点なら反復回数」のままで、
   正答性・誤差が優先されない。
2. ステップ4の完全性チェックが「全83問題」のままである(実際は93問題)。
3. ステップ6で環境変数が無かった場合の報告義務が書かれていない。

## 貼り付け用プロンプト

```text
リポジトリ enomotokan/enomoto_solver の NETLIB ベンチマーク自己改善ループの1反復を実行せよ。
運用ルールの正本は docs/loop-design.md である。作業前に必ず読むこと。

前提: 作業開始時に `git fetch origin main` し、`main` の HEAD 上の
scripts/benchmark_results.json と scripts/loop_state.json を状態の正本として読む
(フィーチャーブランチや未マージPRの状態ファイルを参照しない)。
ベンチマーク実行には環境構築が必要: `python -m venv .venv && .venv/bin/pip install maturin highspy pytest`、
`source .venv/bin/activate && maturin develop --release`、
`python scripts/setup_netlib_data.py`(NETLIBデータ取得、1回のみ、ネットワーク必要)。

0. loop_state.json 内の "loop_iteration_count" を確認せよ(存在しなければ 0)。
   この値を1増やして保存する。これは自己再発火ループ全体の総反復回数である。

1. まず、以前の更新でバグが発生していればその修正を行う。
   次に、対象問題の選定は **`python scripts/select_problem.py` を実行し、その出力した
   問題をそのまま対象とする**。基準の優先順位は「正答性 > 誤差 > 実行時間」であり、
   実装とテスト(tests/test_select_problem.py)がこの順序を固定している
   (詳細は docs/loop-design.md ルール1b)。プロンプト上で独自の基準に選び直さないこと。
   スクリプトが `exhausted`(全問題が連続失敗上限に達した)を報告した場合は、その事実を
   明示的に報告したうえで進めること。

2. fable-bottleneck-analyst サブエージェント(model: claude-fable-5-1)を呼び出し、
   選定された段(tier)に応じた分析を行わせる:
   - correctness の場合: どの段階(presolve のどのルール / フェーズ1 / 双対単体法)で
     誤判定・クラッシュが発生しているかを計測で特定させる。推測で断定させない
   - accuracy の場合: 目的関数値がずれる原因(許容誤差、スケーリング、悪条件、
     presolve の後処理)を特定させる
   - runtime の場合: プロファイリングを行い、HiGHS と実行時間・反復回数・presolve縮約率を
     比較し、「presolve不足 / 反復数過多 / 1反復が重い」の3分類のどれかに落とさせる
   いずれの場合も分析結果を analysis/<問題名>_<日時>.md に保存させる。

3. その分析結果に基づき実装を修正せよ。変更は最小限にし、既存のアーキテクチャ・APIを
   壊さないこと。

4. NETLIB の**全93問題**でベンチマークを再実行し
   (`python -m enomoto_solver.benchmark_highs --max-vars 100000000`)、
   benchmark_results.json を更新せよ。更新後、結果ファイル内のエントリ数を実際に
   カウントするコマンド(jq や python -c 等)を実行し、その出力が **93** であることを
   明示的に確認・報告すること。目視判断や「概ね揃っている」という曖昧な判断は禁止する。
   93と一致しない場合は、以降のPR作成・コミットを一切行わず、直ちに report/ に原因を
   記録して終了すること。
   (93問題の定義は docs/loop-design.md ルール3。93問題が揃うとは全問題の「エントリが
   存在する」ことであり、全問題が最適解に到達することではない。)

5a. 改善が確認できた場合: claude/<問題名>-<日時> ブランチにコミットし、分析サマリ・
    ベンチマーク差分を含むPRを作成した上で、**そのPRを人間のレビュー待ちにせず
    ループ自身が直ちに main へマージする**(docs/loop-design.md ルール2、
    2026-09-21改定)。マージ後、マージ済みの main 上で計測した結果を
    benchmark_results.json に反映し、loop_state.json の連続失敗カウントを0に
    リセットする。「改善」の定義は選定された段に対応する(docs/loop-design.md
    ルール4): correctness=誤判定/クラッシュの解消、accuracy=相対差が許容値以下、
    runtime=ours/highs比の有意な改善。いずれの場合も**他の92問題に退行がないこと**
    (状態が悪化した問題が無いこと)が前提条件である。

5b. 改善が確認できない場合(93問題揃わなかった場合を含む): 変更をすべて破棄し、
    loop_state.json の連続失敗カウントを+1し、report/<問題名>_<日時>.md に記録する
    (report/ が無ければ作成する)。

    5a・5b いずれの場合も、状態ファイル(loop_state.json / benchmark_results.json)は
    コードのPRとは独立に、それらのみを含む単独コミットとして `main` へ直接 push する
    (docs/loop-design.md ルール2)。不完全なベンチマーク結果で benchmark_results.json を
    上書きしてはならない。

6. loop_state.json の "loop_iteration_count" が10以上になった場合は、ここでループを終了せよ。
   次の反復は起動せず、report/loop_stopped_<日時>.md に「反復回数上限(10回)に到達した
   ためループを停止した」旨と、直近の各問題の状況サマリを記録して終了せよ。

   "loop_iteration_count" が10未満の場合のみ、この1反復を終了する前に、環境変数
   ROUTINE_API_URL / ROUTINE_API_TOKEN を使ってこのルーチン自身のAPIエンドポイントに
   POSTリクエストを送信し、次の反復をただちに起動せよ。これによりループを継続する。
   これらの環境変数が存在しない場合は、次の反復を起動できなかった事実を明示的に報告すること
   (黙って終了しない)。

いずれの場合も、依存関係の破壊的更新やCI設定の変更など取り消し困難な変更は行わないこと。
```
