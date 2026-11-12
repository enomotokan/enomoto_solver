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

## C. 58 問 × 種 1〜3 × 60 秒、試作 cpr (`ENOMOTO_MIP_CLIQUE_CUTS=probe_r`) (2026-11-12 に足した、6 分担、それぞれ約 1 時間。**コミット「CLIQFB: probe_r で使わないときは衝突グラフも捨てる」以降でビルドすること**。それより前のビルドで回し始めた分は p58r_k*.jsonl の名前のまま残し、別に回し直す)
cps は「使わない」判定の問題でも比べの状態から続けるため道筋が変わり、roll3000 が種 1〜3 とも遅くなった (23〜33 秒 → 49〜60 秒)。
cpr は使わないときに比べの状態を捨てて既定と同じ道筋にする (roll3000 は 22〜25 秒で既定と同じ)。A・B (cps) はそのまま終えてよい。
| 分担 | 命令 |
|---|---|
| C0 | `python run.py p58r2_k0.jsonl def,cpr 1,2,3 60 @k0/6` |
| C1 | `python run.py p58r2_k1.jsonl def,cpr 1,2,3 60 @k1/6` |
| C2 | `python run.py p58r2_k2.jsonl def,cpr 1,2,3 60 @k2/6` |
| C3 | `python run.py p58r2_k3.jsonl def,cpr 1,2,3 60 @k3/6` |
| C4 | `python run.py p58r2_k4.jsonl def,cpr 1,2,3 60 @k4/6` |
| C5 | `python run.py p58r2_k5.jsonl def,cpr 1,2,3 60 @k5/6` |

## D. 8 問 × 種 1・2 × 600 秒、試作 cpr (B の続き。B の def と比べられるよう cpr だけ回す、4 分担)
| 分担 | 命令 |
|---|---|
| D0 | `CF_LOG=1 CF_LOGDIR=logs600r2 python run.py cq600r2_k0.jsonl cpr 1,2 600 rococoC10-001000 drayage-100-23` |
| D1 | `CF_LOG=1 CF_LOGDIR=logs600r2 python run.py cq600r2_k1.jsonl cpr 1,2 600 neos-1456979 drayage-25-23` |
| D2 | `CF_LOG=1 CF_LOGDIR=logs600r2 python run.py cq600r2_k2.jsonl cpr 1,2 600 hypothyroid-k1 neos859080` |
| D3 | `CF_LOG=1 CF_LOGDIR=logs600r2 python run.py cq600r2_k3.jsonl cpr 1,2 600 30n20b8 net12` |
