"""fracstall の広い計測の設定 (analysis/fracstall_20261122)。def: 今の既定。sf: 試作 `ENOMOTO_MIP_CUT_STALL_FRAC=s` (根のカットのループの停滞の判定で、
分数の整数列の本数の代わりに LP 解の整数列の距離の総和 F を、このループのこれまでの最小 F と比べる)。どれも 4 スレッド (run.py の TH)。"""
CFGS = {"def": {}, "sf": {"ENOMOTO_MIP_CUT_STALL_FRAC": "s"}}
