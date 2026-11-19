"""P2-PAR (4 スレッドの方が遅い問題、README.md) の設定。def: 今の既定、4 スレッド。
p2b: 試作 ENOMOTO_MIP_P2BASIS=1 (前処理の LP による固定の最初の LP の基底を、手間の見積りが閾値以上でも実際の秒が 1 秒未満なら根の LP に渡さない)、4 スレッド。
p2rrc: ENOMOTO_MIP_P2RRC=0.9 (既定にした。根の被約費用のサブ MIP を、伝播の後に自由に残る列の割合が 0.9 以上なら始めない)、4 スレッド。"""
CFGS = {
    "def": {},
    "p2b": {"ENOMOTO_MIP_P2BASIS": "1"},
    "p2rrc": {"ENOMOTO_MIP_P2RRC": "0.9"},
}
