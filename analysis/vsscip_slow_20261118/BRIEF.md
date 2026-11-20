# SCIP より遅い問題の調査・対策 (2026-11-18、利用者の依頼「SCIP との比較が終わったら、遅い問題の調査と対策の立案、実装、検証」)

元の計測: analysis/vsscip_20261117/res_M_s*.jsonl (Mittelmann 186 問 × 種 1 × {def = 当方既定 4 スレッド, def_t1 = 1 スレッド, scip = SCIP 10.0.2 逐次}、600 秒)。
集計 (45/50 分担時点): def/scip −38.0% [−60.9, −1.8]、解けた数 def 64・def_t1 43・scip 50。def_t1/scip +24.5% [−23.7, +105.6]。
ビルドの注意: 計測の版 a9d7bae3d とその後の版には、ノードの LP の「際どい実行不能」の確かめ (558d7a4ce・ec54d0cb0) が入っており、
大きい係数の問題 (rocI-4-11) で LP 値なしの分枝が連鎖して大きく遅くなる不具合がある (WRONG セッションが直し中)。調べる問題で
結果の行の lpinf やログに解き直しが多いなら、比べるときは `ENOMOTO_MIP_NO_LPINF_CHECK=1` も併せて見ること。

## 共通の約束
- 返答・コメント・コミット・記録は日本語 (既存の書き方に合わせる)。作業ブランチは ccr-1beed81b-ex0uhq のみ (push 拒否時は fetch+merge、rebase・force-push はしない)。
  PR は作らない。報告は親セッション session_01HrNYnHHEAD5vXdU4MHQrtc に send_message で日本語で。
- 利用者の方針: パラメータは問題の特徴の連続 (シグモイド) 関数。予算は時間でなく作業量で。採用基準は SGM の 95% 上限 ≲ +5%、解けた数を減らさない、
  全種で 1.25 倍超の遅れなし、誤答 0、cargo test 通過。
- SCIP は `pip install pyscipopt` (6.3.0 = SCIP 10.0.2)。`scripts/run_miplib_benchmark.py --worker-scip` の使い方は analysis/vsscip_20261117/README.md。
  SCIP のパラメータを 1 つずつ切って何が決め手かを確かめる (HANG が fhnw-binpack4-4 で misc/usesymmetry、neos859080 で conflict/useprop を特定した方法) のが有効。
- 進め方: (1) 原因の特定 (SCIP の決め手の切り分け + 当方のログ・プロファイル)、(2) 対策の立案を親に短く報告、(3) 環境変数つきで試作 (既定は不変)、
  cargo test (`LD_LIBRARY_PATH=/usr/local/lib cargo test --release --config 'target.x86_64-unknown-linux-gnu.rustflags=["-C","link-arg=-L/usr/local/lib","-C","link-arg=-lpython3.13"]'`)
  と fuzz (analysis/release019_20261117/fuzz.sh など) で誤答 0、(4) 対象の問題で効くか確かめ、(5) 広く測る台本 (analysis/vsscip_slow_20261118/<名前>/、
  slow2_20261117 の台本の写し、P = MIPLIB 58 問 × 種 1〜3・60 秒、M = slow2 の M 48 問 × 種 1・2・600 秒) を用意して親に計測を依頼。
- 他の担当と重ならないように: fhnw-binpack4-4・neos859080・時間制限の超過は HANG、rocI-4-11 (LPINF) と widden は WRONG、mzzv11・mzzv42z・unitcal_7 は SLOW2、
  CMS750_4 は pcd (前処理の集約、ENOMOTO_AGG_PIVOT_COL + AGG_DUALFREE_INEQ) で 2 種とも 0.5 倍前後に効く兆しがあるので、その結果も踏まえること。
- 他のセッションも同じブランチで並行に作業する。自分の環境変数とファイルだけを触り、共有のコードは差分を小さく、push 前に fetch+merge してビルドとテストを通す。

## 群 (番号 = セッション)
1. SCIP が数秒で決着する問題 (前処理・伝播・対称性・衝突の差): graph20-20-1rand (scip 4.6 秒、当方は 2 設定とも 600 秒)、neos-787933 (scip 2.3 / def 291.7 / t1 TL)、
   bnatt400 (scip 141.6、当方は解なし・下界 0)、bnatt500 (3 者とも解なし)、neos-3381206-awhea (scip 1.2 / def 3.2 / t1 18.5)。
2. 木の効率 (ノード・分枝・カット): cod105 (scip 44.9 / def 144.9 / t1 146.9)、istanbul-no-cutoff (110.6 / 298.9 / TL)、fast0507 (350.6 / TL / TL)、
   dano3_3 (96.9 / 224.2 / 186.1、4 スレッドが 1 スレッドより遅い。dano3_5 も 285 / 456 / 303)、supportcase42 (全員 TL、当方は 600 秒で 920 ノード・下界 7.344、scip は 29489 ノード・7.751)、
   seymour (全員 TL、下界 scip 416.69 / def 412.0)。
3. 小さい問題の手間: hypothyroid-k1 (scip 16.3 / def 43.2 / t1 39.9)、neos8 (2.2 / 5.2 / 7.6)、exp-1-500-5-5 (3.0 / 6.9 / 5.8)、swath1 (11.6 / 14.6 / 55.7)、
   seymour1 (67.7 / 85.3 / 210.0)、beasleyC3 (28.4 / 32.8 / 74.9)。
4. 解探し (時間切れ時の暫定解が SCIP よりずっと悪い): momentum1 (scip 115639 / def 527463)、brazil3 (80 / 243)、neos-5107597-kakapo (3672 / 13248、下界も 1172.7 / 262)、
   neos-5195221-niemur (0.0039 / 0.0117、下界も 3.1e-4 / 2.3e-5)、app1-2 (scip −23 / 当方は解なし)、csched008 (173 / 182)、sorrell3 (−16 / −13)。
5. LP・根・メモリ: neos-3988577-wolgan (def_t1 は根の LP が 600 秒で終わらない)、neos-2746589-doon (def は 600 秒で根から進まずノード 0、t1 は 83・scip 1046)、
   n3div36 (def の RSS 5457 MB、t1 は 767 MB)、uccase12 (def 2918 MB / scip 1457 MB)。

## 追記 (2026-11-18、利用者の方針の訂正)
- 「なんでもシグモイドにするのではなく、普通の if で充分ならそれでいい」。特徴で入り切りするとき、効く問題と効かない問題がはっきり分かれ境目の近くに問題がないなら、しきい値の if で書く。境目の近くに問題が多い・効き目が特徴とともに徐々に変わる場合にだけ連続 (シグモイド) にする。
