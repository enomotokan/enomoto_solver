# SMALLFIX の確かめの手順 (2026-11-13)

試作 C3 (`ENOMOTO_MIP_CLQFB_CAP`) と D1 (`ENOMOTO_MIP_TRNOFIX`) を cand1114 の上で測る。中身と結果は analysis/smallfix_20261113.md。
設定は `cfg.py` (base = CAND1114、c3 = + `CLQFB_CAP=st`、d1 = + `TRNOFIX=1`、c3d1 = 両方)。

## 分担 (分担 1 つ = セッション 1 つ、どれも 1.5 時間以内の見積り)

固定するコミット: このファイルを足したコミット (分担の依頼に書く)。

```sh
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq
git checkout --detach <固定するコミット>
pip install maturin highspy==1.15.1 && pip install -e .      # リリースのビルド (20〜25 分)
python analysis/smallfix_20261113/run.py {P|H} K              # 途中から再開できる。ソルバーは同時に 1 つ (4 スレッド)
```

- P (58 問 x 種 1〜3 x {base, c3, d1, c3d1} 60 秒): 6 分担 (K = 0〜5、見積り各 1.19 時間)。同じ (問題, 種) の 4 設定は同じ分担。
- H (HIGHSLOSS の 12 問 x 種 1 x {base, c3d1} 600 秒): 4 分担 (K = 0〜3、見積り各 1.02 時間)。同じ問題の 2 設定は同じ分担。
- 結果は `res_{P|H}_sK.jsonl` (1 回 1 行、`cnt` に木の中の再スタートの数・固定 0 列の数・TRNOFIX で止めた数・clique fallback の秒)、
  再スタートと clique fallback の行だけのログは `logs_{P|H}/<問題>.<設定>.s<種>.txt.gz`。両方をコミットする
  (名指しの git add、`git pull --rebase --autostash` の後 push)。
- 誤答の確かめは `bash analysis/smallfix_20261113/fuzz.sh` (このセッションで回す)。

## 追加: d1g (2026-11-13、`ENOMOTO_MIP_TRNOFIX=g`)

- Q (58 問 x 種 1〜3 x {base, d1g} 60 秒): 3 分担 (K = 0〜2、見積り各 1.19 時間、`plan_q.py`)。`python analysis/smallfix_20261113/run.py Q K`。
  コンテナの再起動などで止まっても、同じ命令で `res_Q_sK.jsonl` の済んだ回を飛ばして続きから再開する。
- 手元: `run.py L2 0` (supportcase26・timtab1・noswot x 種 1〜3 x {base, d1, d1g} 60 秒と rococoC10-001000 x 種 1・2 x {d1g, base} 600 秒)、
  `FUZZCFG=d1g bash analysis/smallfix_20261113/fuzz.sh`。
