# 試作 `ENOMOTO_MIP_CLQ_LAZY` の対の比べ (2026-11-17、treeclose 9.80 節)

余裕のあるクリーク型の行を木に入るときに LP から外し、ノードの LP 解が破ったらカットの行として戻す試作 (lazy) を既定 (def) と比べる。
同じ問題・種の 2 設定は同じ分担 (= 同じ機械) で続けて解く。**カットの共有を既定にした 1005b567d 以降 (この台本のコミット以降) でビルドして回す。**

- P: 58 問 × 種 1〜3 × {def, lazy}、4 スレッド・60 秒 (348 回)、6 分担 (見積り各約 0.5 時間): `python run.py P K` (K = 0〜5)
- M: mzzv11・neos-3216931-puriri・neos-933966・supportcase33・irp・satellites2-60-fs × 種 1・2 × {def, lazy}、600 秒 (24 回)、6 分担 (見積り各約 0.7 時間): `python run.py M K` (K = 0〜5)
- 集計 `python summ.py P > summ_P.txt`、`python summ.py M > summ_M.txt`。
- `local.py` / `res_local.jsonl`: 手元の確かめ (6 問 × {def, lazy}、4 スレッド・種 1)。
