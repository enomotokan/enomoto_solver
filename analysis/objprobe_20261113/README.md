# OBJPROBE の計測の分担 (2026-11-13)

試作 `ENOMOTO_MIP_OBJPROBE` (既定では切) の重い計測。中身と結果は ../objprobe_20261113.md。設定は cfg.py:

| 名札 | 環境変数 (cand1114 = ../cand1114_20261113/cfg.py の CAND1114 に足す) |
|---|---|
| base | なし |
| c1 | `ENOMOTO_MIP_OBJPROBE=pc` (根のカットのループの中の目的の上限つきプロービング + ラウンドごとの被約費用の固定) |
| c2 | `ENOMOTO_MIP_OBJPROBE=r` (根のカットの各ラウンドの乱択丸め) |
| c12 | `ENOMOTO_MIP_OBJPROBE=rpc` |
| c12h | `ENOMOTO_MIP_OBJPROBE=rpch` (+ 根のカットの各ラウンドの軽いヒューリスティクスの束、追加の試作) |

## 手順 (分担 1 つ = セッション 1 つ)

固定するコミット: このファイルを足したコミット (分担の依頼に書く)。

```sh
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq
git checkout --detach <固定するコミット>
pip install maturin highspy==1.15.1 && pip install -e .      # リリースのビルド (20〜25 分)
python analysis/objprobe_20261113/run.py {P|H} K              # 途中から再開できる。ソルバーは同時に 1 つ (4 スレッド)
```

- P (58 問 × 種 1〜3 × {base, c1, c2, c12} 60 秒、696 回): 4 分担 (K = 0〜3、見積り各 1.32 時間、cand1114 の P の回の壁時計から)。
  同じ (問題, 種) の 4 設定は同じ分担。
- H (HIGHSLOSS の 12 問 × 種 1 × {base, c12} 600 秒、24 回): 3 分担 (K = 0〜2、見積り各 1.02〜1.09 時間)。同じ問題の 2 設定は同じ分担。
- 追加 (h の確かめ、固定するコミットは d24ffbeb (乱択丸めの乱数列を分けた後)。b79bbaee で始めた分担はそのまま続けてよい):
  - Q (58 問 × 種 1〜3 × {base, c12, c12h} 60 秒、522 回): 3 分担 (K = 0〜2、各 1.32 時間)。`python analysis/objprobe_20261113/run.py Q K`
  - G (12 問 × 種 1 × {base, c12, c12h} 600 秒、36 回): 5 分担 (K = 0〜4、各 0.88〜1.02 時間)。`python analysis/objprobe_20261113/run.py G K`
- 終わったら res_{P|H|Q|G}_sK.jsonl だけを名指しで add してコミットし、`git pull --rebase --autostash` の後 push する (`git add -A` は使わない)。
- 集計: `python analysis/objprobe_20261113/summ.py P > summ_P.txt` (H・Q・G も同じ)。
- 誤答の確かめ (済み、この分担には入らない): `bash analysis/objprobe_20261113/fuzz.sh` (結果は ../objprobe_20261113.md)。
