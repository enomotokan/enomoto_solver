"""群 5 (LP・根・メモリ、analysis/vsscip_slow_20261118/g5/README.md) の設定。
def・def_t1: 0.1.9 の既定 (4 スレッド・1 スレッド)。
inh: 試作 ENOMOTO_MIP_G5INH=pc (TC の引き継ぎの鎖を共有の待ち行列へ付けない + 長い鎖に足さない)、4 スレッド。def と対にする。
can: 試作 ENOMOTO_IPM_G5CANCEL=1 (内点法の正規方程式・拡大系を作る途中でも止めの合図を見る)、4 スレッド。def と対にする。
fin: 試作 ENOMOTO_MIP_G5FIN=1 (時間切れの後始末で木の従のスレッドが返すノードの引き継ぎの鎖を展開しない)、4 スレッド。def と対にする。
fin0: ENOMOTO_MIP_G5FIN=0 (G5FIN を既定にした 9bf59a9b0 以降で従来の展開に戻す)、4 スレッド。
seq_t1: 試作 ENOMOTO_MIP_G5SEQ=1 (1 スレッドの大きい基底なしの求解を二段解法 → 内点法 + クロスオーバーの順に)、反復の上限の係数 0.5、1 スレッド。def_t1 と対にする。"""
CFGS = {
    "def": {},
    "def_t1": {},
    "inh": {"ENOMOTO_MIP_G5INH": "pc"},
    "can": {"ENOMOTO_IPM_G5CANCEL": "1"},
    "fin": {"ENOMOTO_MIP_G5FIN": "1"},
    "fin0": {"ENOMOTO_MIP_G5FIN": "0"},
    "seq_t1": {"ENOMOTO_MIP_G5SEQ": "1", "ENOMOTO_T_MIP_G5SEQ_C": "0.5"},
}
