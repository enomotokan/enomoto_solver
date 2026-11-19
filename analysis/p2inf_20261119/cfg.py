"""P2-INF の設定 (analysis/p2inf_20261119/README.md)。名前が _t1 で終われば 1 スレッド (run.py の TH)。
- def: このコミットの既定。
- rowtiny: LP の実行不能の証明の際どさを測るとき、丸めの誤差ほどの重みの行を射線から落とす (ENOMOTO_MIP_LPINF_ROWTINY=1)。
  既定では片側が無限の行に 1e-17 程度の重みが載ると正しい証明を際どいとみなし、ノードを刈らない・強分枝の子を不明にする (README 2 節)。
- sstfeas / sstfeas_t1: 目的が全部 0 の問題に限って対称性を SST のカットで扱う (ENOMOTO_MIP_SYM_SST_FEAS=1、README 4 節)。def_t1 と対。"""
CFGS = {
    "def": {},
    "rowtiny": {"ENOMOTO_MIP_LPINF_ROWTINY": "1"},
    "def_t1": {},
    "sstfeas": {"ENOMOTO_MIP_SYM_SST_FEAS": "1"},
    "sstfeas_t1": {"ENOMOTO_MIP_SYM_SST_FEAS": "1"},
}
