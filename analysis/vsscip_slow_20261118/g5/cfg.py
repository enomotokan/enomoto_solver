"""群 5 (LP・根・メモリ、analysis/vsscip_slow_20261118/g5/README.md) の設定。
def・def_t1: 0.1.9 の既定 (4 スレッド・1 スレッド)。
inh: 試作 ENOMOTO_MIP_G5INH=pc (TC の引き継ぎの鎖を共有の待ち行列へ付けない + 長い鎖に足さない)、4 スレッド。def と対にする。
seq_t1: 試作 ENOMOTO_MIP_G5SEQ=1 (1 スレッドの大きい基底なしの求解を二段解法 → 内点法 + クロスオーバーの順に)、反復の上限の係数 0.5、1 スレッド。def_t1 と対にする。"""
CFGS = {
    "def": {},
    "def_t1": {},
    "inh": {"ENOMOTO_MIP_G5INH": "pc"},
    "seq_t1": {"ENOMOTO_MIP_G5SEQ": "1", "ENOMOTO_T_MIP_G5SEQ_C": "0.5"},
}
