"""P2-PAR (4 スレッドの方が遅い問題、README.md) の設定。def: 今の既定、4 スレッド。
p2b: 試作 ENOMOTO_MIP_P2BASIS=1 (前処理の LP による固定の最初の LP の基底を、手間の見積りが閾値以上でも実際の秒が 1 秒未満なら根の LP に渡さない)、4 スレッド。"""
CFGS = {
    "def": {},
    "p2b": {"ENOMOTO_MIP_P2BASIS": "1"},
}
