# カットのプールの共有 (sc) と待ち行列の釣り合わせなし (nb) の対の比べ (2026-11-17、treeclose 9.77 節)

共有の診断 (`share_20261117`) で、カットのプールの共有 (`ENOMOTO_MIP_TREE_SHARE_CUTS`) が kasai 4 回とも既定より速く (105・126 秒 対 160・240 秒)、
csched008 の種 2 を 380 秒で解いた。釣り合わせなし (`ENOMOTO_MIP_TREE_NO_BALANCE`) も M で 0.73 倍 (機械の差込み)。P の 6 問では両方とも揺れの範囲。
58 問と重い問題で既定と比べる。どちらも既存の診断で、コードは変えない。

- P: 58 問 × 種 1〜3 × {def, sc, nb}、4 スレッド・60 秒 (522 回)、6 分担 (見積り各約 0.7 時間): `python run.py P K` (K = 0〜5)
- M: kasai・csched008・sct2・tbfp-network・irp・mzzv11 × 種 1・2 × {def, sc, nb}、600 秒 (36 回)、6 分担 (見積り各約 1 時間): `python run.py M K` (K = 0〜5)
- 同じ問題・種の 3 設定は同じ分担 (= 同じ機械) で続けて解く。集計 `python summ.py P > summ_P.txt`、`python summ.py M > summ_M.txt`。
