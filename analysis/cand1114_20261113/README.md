# cand1114 の確かめ (2026-11-13)

束 cand1114 = cfg.py の `CAND1114`。cand1113 (analysis/cand1113_20261112/) を、その確かめと leave-one-out (loo.md・loo2.md) の結果で組み直した既定化の候補。
誤答の直し 40e80904 (unitcal_7、TREE2_FORCEQ の許容誤差) と UNB_FEAS (dc1f6842) は既定に入っているので、def・cand1114 の両側に入る。

## 組み直し (cand1113 → cand1114)

| 項目 | cand1113 | cand1114 | 理由 |
|---|---|---|---|
| RCSTRONG | irk | **k** | RCSTRONG2 (親の指示。irk は rmatr100-p10 を遅くする) |
| HNFPRE=0 (prune1112) | 入れる | **外す** (既定の HNF 前処理 `bounds` に戻す) | enlight_hard を 0.02 秒 → 10〜37 秒にする (loo2)。fuzz_slack seed 240 の弱い状態もこれだけで起きる (cand1113/fuzz.md) |
| SUBFIX=0 (prune1112) | 入れる | **外す** | neos-960392 を 4 種とも 2 → 3〜6 秒にする (loo、種 1〜4)。ほかに効く問題が見つかっていない |
| IPMRC=utw | 入れる | **外す** | 下の表 |
| DUALEQ | 入れる | **外す** | 下の表 |
| CLQPRB=1d | 入れる | **外す** | 下の表 |
| ほかの 15 項目 (ewk・tenc・n05・net・LMBOOST・prepar・ncm・ns250s・prune1112 の残り 3 つ・ROOTSTOP・LMIP_SWAP・SYMPRE・PLSB・GAP1・GAP4・OBJINT) | 入れる | そのまま | — |

### 迷った 3 項目の得失 (数字は cand1113 の P (58 問 60 秒) と M (Mittelmann 600 秒)、loo.md・loo2.md、各項目の記録から)

| 項目 | 外すと速くなる問題 (害) | 外すと遅くなる問題 (効き) | 判断 |
|---|---|---|---|
| IPMRC=utw | **neos-1122047**: M で def 2.3/2.4 秒 → cand1113 13.7/13.1 秒。utw だけ外すと 2.8/2.6 秒 (loo2、ほかの -X は 13〜14 秒) | 合格の記録 (ipmrc_20261111 §3) で 58 問 −1.5% [−5.7%, +2.3%]。区間は 0 をまたぎ、2 種そろって速い問題はない。Mittelmann の重い 6 問でもそろった効きはない (§4、`uw`)。loo の 7 問・loo2 の 3 問でも、外して 2 種とも遅くなる問題はない | **外す**。示された害が 1 問 (5.7 倍) あり、効きは区間の中 |
| DUALEQ | **binkar10_1**: P で 1.45 倍 (4 種そろって)。DUALEQ だけ外すと種 1〜4 で def 並み (loo、追加の確かめ)。1 列を消すだけで根の切除平面の打ち切りが 13 周目まで遅れ、ノードが 2.3k → 3.8〜5.0k に増える。unitcal_7 の誤答は TREE2_FORCEQ の誤りだが、表に出したのは DUALEQ の作る等式行 (review2) | 合格の記録 (presolve2 §結論 2) で 58 問 +0.1% [−3.4%, +3.2%]。前処理が変わるのは 58 問のうち 5 問だけ。目標の neos859080 は M で def 48.6/3.0 秒、cand1113 2.3/55.3 秒と種で逆向き (揺れ) | **外す**。害は 1 問だが原因まで分かっており、効きは示されていない |
| CLQPRB=1d | **roll3000**: M で 24.8/25.8 → 51.8/67.9 秒、P で 1.4 倍。CLQPRB を外すと種 2 の時間切れが消える (loo。PLSB・ns250s と重なっている)。**cbs-cta**: M で 6.7/6.8 → 18.1/18.9 秒、CLQPRB だけ外すと種 1〜4 で def 並み (loo2)。**neos-3216931-puriri**: M で def は 2 種とも解あり (81380・81480)、cand1113 は 2 種とも解なし (clqprb §結論 7 と同じ型)。air03 1.2〜1.5 倍 (clqprb §結論 6) | **neos8**: P で 0.60 倍、M で 6.4/6.0 → 3.6/3.4 秒。外すと def 並み (loo、ほかの -X は変わらない)。**mzzv11**: 記録では 2 種とも時間切れ → 574・552 秒で解ける (clqprb §結論 7) が、cand1113 の M では 2 種とも時間切れのまま (下界 −21866・−21930 → −21798・−21748 と良くなるだけ)。mzzv42z は M で 342/231 → 199/211 秒だが、CLQPRB の分かは切り分けていない | **外す**。はっきりした害が 3 問 (うち HiGHS に負けている puriri で解を失う)。はっきりした効きは neos8 の 1 問で、目標の mzzv11 は束の中では解けていない。HiGHS との差を詰める項目なので、roll3000・cbs-cta・puriri の害の理由を調べてから入れ直す |

外した 3 項目の効き (neos8 の速さ、mzzv11・mzzv42z) が消えるかどうかは、M の組で確かめる (下)。

## 手順 (分担 1 つ = セッション 1 つ)

固定するコミット: このファイルを足したコミット (分担の依頼に書く。40e80904・dc1f6842 を含む ccr-1beed81b-ex0uhq の先頭)。

```sh
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq
git checkout --detach <固定するコミット>
pip install maturin highspy==1.15.1 && pip install -e .      # リリースのビルド (20〜25 分)
python analysis/cand1114_20261113/run.py {M|P} K              # 途中から再開できる。ソルバーは同時に 1 つ (4 スレッド)
bash analysis/cand1114_20261113/fuzz.sh                       # 分担 F (誤答の確かめ)
```

- P (58 問 × 種 1〜4 × {def, cand1114} 60 秒 + physiciansched6-2 の種 5〜12): 6 分担 (K = 0〜5、見積り各 約 0.8 時間)。同じ (問題, 種) の def と cand1114 は同じ分担。
- M (Mittelmann の 48 問 × 種 1・2 × {def, cand1114} 600 秒 + 種 1 の HiGHS、240 回): 7 分担 (K = 0〜6、見積り各 3.8〜3.9 時間、cand1113 の回の時間から)。同じ問題の回は同じ分担。
  組 (plan.py): cand1113 で 2 種とも遅かった 5 問 (enlight_hard・neos-1122047・cbs-cta・roll3000・neos-960392)、unitcal_7、
  HiGHS に負けている 12 問 (analysis/highsloss_20261113 と同じ)、cand1113 だけが片方の種で解いた問題・2 種とも速かった問題 (cand1113 の summM.txt の一覧)。
- F (誤答の確かめ、1 分担): `fuzz.sh`。cand1113/fuzz.md と同じ組 (fuzz_tiny 400・mip_fuzz_vs_highs 300・fuzz_objint 1000・fuzz_slack 1000、
  いずれも CAND1114 を全部付けて) と `cargo test --release --lib` (既定と CAND1114 の 2 回)。ログは fuzz_logs/。結果は fuzz.md に表で。
  fuzz_slack seed 240 (HNFPRE=0 が原因) が消えていることも見る。
- 終わったら res_{M|P}_sK.jsonl (F は fuzz.md と fuzz_logs/) だけを名指しで add してコミットし push する (`git add -A` は使わない)。
