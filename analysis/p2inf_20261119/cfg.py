"""P2-INF の設定 (analysis/p2inf_20261119/README.md)。名前が _t1 で終われば 1 スレッド (run.py の TH)。
- def: このコミットの既定。
- rowtiny: LP の実行不能の証明の際どさを測るとき、丸めの誤差ほどの重みの行を射線から落とす (ENOMOTO_MIP_LPINF_ROWTINY=1)。
  既定では片側が無限の行に 1e-17 程度の重みが載ると正しい証明を際どいとみなし、ノードを刈らない・強分枝の子を不明にする (README 2 節)。
- cfx: 衝突を分枝と衝突解析に効かせる組 (ENOMOTO_MIP_LIT_EXPLAIN=1・ENOMOTO_MIP_HIGHS_CONFLICT_SCORE=1・ENOMOTO_T_MIP_CONFLICT_WEIGHT=0.1、README 4.11 節)。
- cfx2: cfx の衝突の重みを、擬似費用の利得が正だった観測が 5% 以下 (手がかりなし) のときだけ効かせる形
  (ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT=0.1、README 4.13 節)。
- sstfeas / sstfeas_t1: 目的が全部 0 の問題に限って対称性を SST のカットで扱う (ENOMOTO_MIP_SYM_SST_FEAS=1、README 4 節)。def_t1 と対。
- cwr: 衝突の重みを解いている途中で上げるシグモイドの形 (ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP=0.1) と、上げている間だけの LIT_EXPLAIN
  (ENOMOTO_MIP_LIT_EXPLAIN=auto、README 5 節)。上がらない問題は既定と同じ動き。"""
CFGS = {
    "def": {},
    "rowtiny": {"ENOMOTO_MIP_LPINF_ROWTINY": "1"},
    "def_t1": {},
    "cfx2": {"ENOMOTO_MIP_LIT_EXPLAIN": "1", "ENOMOTO_MIP_HIGHS_CONFLICT_SCORE": "1", "ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT": "0.1"},
    "cfx": {"ENOMOTO_MIP_LIT_EXPLAIN": "1", "ENOMOTO_MIP_HIGHS_CONFLICT_SCORE": "1", "ENOMOTO_T_MIP_CONFLICT_WEIGHT": "0.1"},
    "sstfeas": {"ENOMOTO_MIP_SYM_SST_FEAS": "1"},
    "cwr": {"ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP": "0.1", "ENOMOTO_MIP_LIT_EXPLAIN": "auto"},
    "sstfeas_t1": {"ENOMOTO_MIP_SYM_SST_FEAS": "1"},
}
