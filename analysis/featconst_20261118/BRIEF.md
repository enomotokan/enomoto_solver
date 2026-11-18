# 定数を問題の特徴の関数にする (2026-11-18、利用者の依頼「今まで定数で固定していたが、問題の特徴によって変えた方がよいもの」→「進めて」)

過去の記録で「問題によって良い値が逆向き」の証拠がある MIP 側の定数 (`tunable!("ENOMOTO_T_MIP_...")`) の上位 3 件を、シグモイドの関数にする試作。
洗い出しの根拠は各項に記した記録 (docs/improvement_history.md・analysis/ 以下)。

## 共通の約束 (gate2_20261117/BRIEF.md と同じ)
- 返答・コメント・コミット・記録は日本語。ブランチ ccr-1beed81b-ex0uhq のみ (push 拒否時は fetch+merge、rebase・force-push なし)。PR は作らない。
  報告は親 session_01HrNYnHHEAD5vXdU4MHQrtc に send_message で日本語で。
- パラメータは問題の特徴の連続 (シグモイド) 関数。予算は作業量 (ticks)。採用基準は SGM の 95% 上限 ≲ +5%・解けた数を減らさない・全種で 1.25 倍超の遅れなし・誤答 0・cargo test 通過。
- 進め方: (1) 根拠の記録を読み、今の既定 (記録の多くは 10 月の版。その後 CUTSEL_PROBE・TICKS・CUTLPCAP などが既定になった) の上で、効く側と損する側の
  代表問題を手元で測り直して逆向きがまだあるか確かめる。(2) 特徴を数え (ログや診断から)、両側が分かれるかを確かめ、見込みを親へ短く報告
  (分かれなければここで区切る)。(3) 環境変数つきの試作 (既定は不変)、cargo test・fuzz で誤答 0、代表問題で確かめ。(4) P (58 問 × 種 4〜6・60 秒)・
  M (48 問 × 種 3・4・600 秒) の台本 (slow2 / gate2 の台本の写し) を用意して親に計測を依頼。
- 既定の対称性の扱い (SYMDYN=f) に誤答の弱点が見つかり WRONG が直し中。対称性のある問題で値が変われば、その直しの後で確かめ直す。

## 1. 木の分離の「カットが密」の判定 (`ENOMOTO_T_MIP_NODE_CUT_ADAPT_DENSITY` 10 倍・`ENOMOTO_T_MIP_TREE_SEP_DENSITY_COLS` 0.05)
- 根拠: analysis/node_cut_adapt_fix_20261020 §2 (密度比 = カットの非零 ÷ 元の行の平均: neos-911970 2.9〜6.7 (続けないと時間切れ、3 倍にすると止まって時間切れ)、
  qiu 33〜67・rout 21〜25 (止める側))、analysis/highs_slow_20261106/S3.md §1・§4 (neos-1445765 は判定に掛からず 246 秒、NO_TREE_SEP で 66 秒。
  下限を自由な列の 0.05 倍にすると drayage-25-23 が 5 秒 → 3 回中 2 回時間切れ)、solver.rs のコメント (beasleyC3 は 10 倍で誤って止まった)。
- 特徴の案: ρ = カットの非零 ÷ 元の行の平均 (ρ0 8〜12)、φ = カットの非零 ÷ 自由な列の数 (drayage 5.2%、neos-1445765 32%、φ0 0.15〜0.2)、LP の行の伸び。
  続ける割合 (または TREE_SEP_RATE の倍率) = σ(−k(ln ρ − ln ρ0)) × σ(−k₂(φ − φ0))。
- neos-1445765 は SCIP より遅い群 2 (vsscip_slow_20261118) の対象。群 2 のセッション (session_01HrNYnJCST7ByXHBkH1LzGu) と共有すること。

## 2. カット選択の質の閾値 (`ENOMOTO_T_CUTSEL_MIN_REL` 0.5、関連 `ENOMOTO_T_CUTSEL_MAXPAR` 0.3)
- 根拠: analysis/cutloop_hl_20261022 §1・§5 (roll3000 は 0 で 11349 → 11648、閾値を下げる fb で misc07 2 倍・mik-250 5 → 7 秒)、analysis/sp150x300d_20261015
  (MIN_REL=0 MAXPAR=0.9 で 60 秒切れ → 1.4 秒)、analysis/mittelmann_highs_only_20261016 (B) (sp150x300d・pg・nu25-pr12 25 → 12.6 秒が速く、p200x1188c
  6.5 → 12.5 秒、40 問 SGM +16%)、analysis/roll3000_20261020 (0.2 で mik-250 2 倍)、analysis/slow2_20261117 §7 (unitcal_7 の根の下界は 0.1 で上がる。
  選び方の下見 CUTSEL_PROBE が pool を選んだ回では MIN_REL は効かない §7.1)。
- 特徴の案: 根の LP の双対退化の割合、目的と平行度 0 の候補の割合、目的の列の数、候補数 ÷ 採った本数。MIN_REL = 0.5 × σ(k(s − s0))。
  損する側 (mik-250・misc07・p200x1188c) でこの特徴を測った記録はないので、まず数える。

## 3. 根のカットのループの長さ (`ENOMOTO_T_MIP_CUT_TIME_FRAC` 0.1 (TICKS `w` で手間)・`ENOMOTO_T_MIP_CUT_ROUNDS` 25・`ENOMOTO_T_MIP_ROOT_MAX_CUTS` 50)
- 根拠: analysis/timedep_20261114 §6.2 (上限を 6 にすると h80x6320d ×2.64・neos-911970 ×2.81・exp-1-500-5-5 ×2.03、p200x1188c だけ ×0.43。24 で p200x1188c ×1.52)、
  analysis/mittelmann_highs_only_20261016 (A、100 ラウンド・500 本: neos-3381206-awhea 時間切れ → 6.2 秒、misc07・eil33-2・binkar10_1 速い、
  neos-860300・mik-250・h80x6320d 遅い)、analysis/gate_20261117 B (60 ラウンド・第 2 段階: qnet1 3.5 → 8.7、p200x1188c 2.9 → 4.8、neos17 5.1 → 10.2、
  fiber 0.9 → 1.6、nubu 6.9 → 4.2。「11 問では根の特徴で分ける手がかりが nubu 1 問だけ」)。
- 特徴の案: r = 1 ラウンドの手間 ÷ 根の LP の手間 (または根の LP の手間)、根で閉じる軽い問題か。CUT_TIME_FRAC = a × σ(k(ln r − ln r0))。
  gate B が分けられなかったので、最初に既存の P・M のログから r を数えて分かれるかを確かめる。
- exp-1-500-5-5・neos-3381206-awhea は SCIP より遅い群 3・群 1 の対象。群 3 (session_01HrNYnGtTw7QyKEbYQadiGY) と共有すること。

## 追記 (2026-11-18、利用者の方針の訂正)
- 「なんでもシグモイドにするのではなく、普通の if で充分ならそれでいい」。特徴で入り切りするとき、効く問題と効かない問題がはっきり分かれ境目の近くに問題がないなら、しきい値の if で書く。境目の近くに問題が多い・効き目が特徴とともに徐々に変わる場合にだけ連続 (シグモイド) にする。
