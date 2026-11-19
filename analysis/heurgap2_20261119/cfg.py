"""P2-HEUR の計測の設定。rg: 試作 ENOMOTO_MIP_RENS_BIGGAP=0.5 (根のダイブ式 RENS の解の間隙が 0.5 を超えたら既定の RENS も回す)。
試作が働くのは根のダイブ式 RENS (暫定解があるとき) の道で、4 スレッドの根で暫定解がないときは並列の「暫定解なしの RENS」(固定率 0.8) が
先に回るので、効くのは主に 1 スレッド。そこで M は 1 スレッド (_t1)、P は 4 スレッド (害の確かめ) で測る。"""
CFGS = {"def": {}, "rg": {"ENOMOTO_MIP_RENS_BIGGAP": "0.5"}, "def_t1": {}, "rg_t1": {"ENOMOTO_MIP_RENS_BIGGAP": "0.5"}}
