"""heur1t の広い計測の設定 (analysis/heur1t_20261123)。どちらも 1 スレッド (run.py の TH: 名前が _t1 で終われば 1 スレッド)。
def_t1: 今の既定。fl_t1: 試作 `ENOMOTO_MIP_HEUR1T=fl` (外側の根から出したサブ MIP でも暫定解がなければ robust FJ を回す・
1 スレッドの木の大近傍探索の時間の割合をヒューリスティクス役の割合 0.25 まで上げる)。"""
CFGS = {"def_t1": {}, "fl_t1": {"ENOMOTO_MIP_HEUR1T": "fl"}}
