"""OBJTARGET の計測の設定 (README)。def = 今の既定。l = Local-MIP の目的の行の上限を「最良 − max(刻み, α (最良 − L))」に置く試作 (ENOMOTO_MIP_OBJTARGET=l)。
p = 前処理と並行して PDLP (2000 反復) の点から FJ を回し、解を開始解にする試作 (ENOMOTO_MIP_TPRE=p)。
pl = p の後に PDLP の点から Local-MIP を 10 万手 (ENOMOTO_MIP_TPRE=pl)。"""
CFGS = {"def": {}, "l": {"ENOMOTO_MIP_OBJTARGET": "l"}, "p": {"ENOMOTO_MIP_TPRE": "p"}, "pl": {"ENOMOTO_MIP_TPRE": "pl"}}

# 再スタートの費用の試作 (README の 10 節)。c = ENOMOTO_MIP_PRESOLVE_STOP_FINE (プロービングで止めの合図を候補ごと・窓の中でも見る)、
# b = ENOMOTO_MIP_RESTART_DROP_FIXED (再スタートの問題から固定された列を落とす)、a = ENOMOTO_MIP_RESTART_DENSE_POOL (密なカットをプールへ)。
# 接頭辞 pl_・plw_・vb_ は TPRE=pl・TPRE=plw・VB01=b (SCIP-GATE の vb) と組み合わせる
_RS = {"c": {"ENOMOTO_MIP_PRESOLVE_STOP_FINE": "1"}, "b": {"ENOMOTO_MIP_RESTART_DROP_FIXED": "1"}, "a": {"ENOMOTO_MIP_RESTART_DENSE_POOL": "1"}}
_BASE = {"": {}, "pl_": {"ENOMOTO_MIP_TPRE": "pl"}, "plw_": {"ENOMOTO_MIP_TPRE": "plw"}, "vb_": {"ENOMOTO_MIP_VB01": "b"}}
for _pre, _b in _BASE.items():
    if _pre:
        CFGS.setdefault(_pre[:-1], dict(_b))
    for _v in ["c", "cb", "cba", "b", "a"]:
        _e = dict(_b)
        for _ch in _v:
            _e.update(_RS[_ch])
        CFGS[_pre + _v] = _e
