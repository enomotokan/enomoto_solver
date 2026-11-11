# CLIQFB の計測の分担

どの分担も同じ手順 (リポジトリの根で):
```
sh analysis/cliqfb_20261111/build.sh            # $HOME/cliqfb_pkg に入る
cd analysis/cliqfb_20261111
CF_PKG=$HOME/cliqfb_pkg python run.py <出力> <設定> <種> <時間> <問題>
```
既定 (def) と試作 (cps = `ENOMOTO_MIP_CLIQUE_CUTS=probe_s`、2026-11-11 夜に cp から変えた。もう cp で回し始めた分担はそのまま cp で終えてよい) は同じ分担で、同じ (問題, 種) を続けて回す (run.py がそうする)。
1 つの機械で同時に回すソルバーは 1 つ (4 スレッド)。途中で止まっても同じ命令で続きから回る。終わったら出力の jsonl をコミットして push する。

## A. 58 問 × 種 1〜3 × 60 秒 (6 分担、それぞれ約 1 時間)
| 分担 | 命令 |
|---|---|
| A0 | `python run.py p58_k0.jsonl def,cps 1,2,3 60 @k0/6` |
| A1 | `python run.py p58_k1.jsonl def,cps 1,2,3 60 @k1/6` |
| A2 | `python run.py p58_k2.jsonl def,cps 1,2,3 60 @k2/6` |
| A3 | `python run.py p58_k3.jsonl def,cps 1,2,3 60 @k3/6` |
| A4 | `python run.py p58_k4.jsonl def,cps 1,2,3 60 @k4/6` |
| A5 | `python run.py p58_k5.jsonl def,cps 1,2,3 60 @k5/6` |

## B. クリークの行の多い 8 問 × 種 1・2 × 600 秒 (4 分担、それぞれ最大約 2.7 時間)
ログも残す (`CF_LOG=1 CF_LOGDIR=logs600`)。
| 分担 | 命令 |
|---|---|
| B0 | `CF_LOG=1 CF_LOGDIR=logs600 python run.py cq600_k0.jsonl def,cps 1,2 600 rococoC10-001000 drayage-100-23` |
| B1 | `CF_LOG=1 CF_LOGDIR=logs600 python run.py cq600_k1.jsonl def,cps 1,2 600 neos-1456979 drayage-25-23` |
| B2 | `CF_LOG=1 CF_LOGDIR=logs600 python run.py cq600_k2.jsonl def,cps 1,2 600 hypothyroid-k1 neos859080` |
| B3 | `CF_LOG=1 CF_LOGDIR=logs600 python run.py cq600_k3.jsonl def,cps 1,2 600 30n20b8 net12` |
