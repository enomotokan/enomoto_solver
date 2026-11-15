## 1. 調べ (文献・他のソルバー)

注: arxiv.org などに届かなかったので、論文の数値は抄録・二次要約による。ソルバーは GitHub の現行のソースを読んだ。

- **SCIP** (Achterberg 2007、sepa.c): 分離器ごとに priority・freq・maxbounddist・delay・expbackoff (SCIP 5.0 から、深さ/freq が 4 の冪のときだけ呼ぶ)。
  delay の分離器は他がカットを見つけなかったラウンドだけ呼ぶ。打ち切りは主に `maxstallroundsroot` = 10。`SCIPsepaGetNCutsApplied` などの統計は表示だけで、
  呼ぶかの判断には使わない (効き目によるオンラインの適応はない)。SCIP 8 でカット選択がプラグインになり既定は hybrid、SCIP 9 で ensemble (Turner) と
  dynamic を足したが既定ではない (Bestuzheva ほか 2021、Bolusani ほか 2024)。
- **CBC** (CbcModel・CbcCutGenerator): howOften の負の値 (-98 = 伸びている間、-99 = 根だけ、-100 = 止める)、`switchOffIfLessThan` (根の最初のパスで本数が少なければ止める)、
  根の終わりに「本数の 0.2 x 平均」より少ない生成器を √ 比で間引く (k + 1e6)、0 本は SCANCUTS (1000 ノード) ごとに様子見。serialCuts (ENOMOTO の SEP_ADAPTIVE が移植済み)。
  どれも本数と伸びで見て、分離器ごとの伸びへの寄与は見ない。
- **HiGHS** (HighsSeparation.cpp): 含意 → クリーク → tableau・path・mod-k → プールを固定の順で毎ラウンド全部呼ぶ。伸びが前のラウンドまでの 1.01 倍を超える間繰り返す。
  根は平滑化した進み・停滞 3 回・2√(log2 木の大きさ) ラウンドで止める。分離器ごとの適応はない (numCutsFound は数えるだけ)。
- **Turner, Koch, Serrano, Winkler 2023** (OJMO、Adaptive cut selection): SCIP のカット選択の重み (efficacy・dcd・isp・obp) を問題ごとに GCNN で予測。固定の規則はどれも外す問題の族が
  あることを示し、問題ごとの適応は大きく効くが 1 つの関数で表すのは難しいと結論。
- **Berthold, Francobaldi, Hendel 2022** (Learning to use local cuts): Xpress で局所カットを使うかを回帰森で問題ごとに決める (完全な予言で平均約 11% 速いのが上限)。
- **Tang, Agrawal, Faenza 2020**・**Paulus ほか 2022** (先読み: 1 本足したときの下界の伸びを expert に模倣)・**Huang ほか 2022** (カットの順位、生産計画で 12% 速い)・
  **Wang ほか 2023** (HEM、本数の比率と順序を RL で学ぶ): どれもカット 1 本ずつの選択を事前の学習で決める。
- **Li, Ouyang, Paulus, Wu 2023** (L2Sep、NeurIPS): どの分離器を使うかの構成を学び、求解の途中でも切り替える。SCIP 既定より合成問題で最大 72%、実問題で最大 37% 短い。
  分離器単位の配分にいちばん近いが、事前の学習が要る。
- **Chmiela, Gleixner, Lichocki, Pokutta 2023**: LNS・潜りのヒューリスティクスを bandit で配分 (難しい問題で約 4% 速い)。分離器に当てはめた bandit の研究は見つからなかった (未確認)。
- **Deza, Khalil 2023** (サーベイ)、**Balcan ほか 2021/2022** (カット選択の標本複雑度、未精査)。

示唆 (推測): 報酬は「採ったカットの |双対値| x 違反で下界の伸びを按分」、生き残りを遅れて届く補助の報酬に、費用は時刻でなく決定的な仕事 (非零の数・反復) で割る。
分離器は 10 前後なので ε-greedy か割引の UCB で足り、効かない分離器も止め切らずに指数的に間引いて試し直す (SCIP の expbackoff・CBC の SCANCUTS)。根と木は別の統計にする。
