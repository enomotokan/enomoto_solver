"""gainstall の P・M の設定 (analysis/gainstall_20261122)。def: 今の既定。
w1e-2・w3e-2: 窓で見る停滞の判定 (ENOMOTO_MIP_STALL_GAIN=xw、3 節)。g3e-2: 始めからの伸びで割る形 (1・2 節)。"""
CFGS = {
    "def": {},
    "g3e-2": {"ENOMOTO_MIP_STALL_GAIN": "3e-2"},
    "w1e-2": {"ENOMOTO_MIP_STALL_GAIN": "1e-2w"},
    "w3e-2": {"ENOMOTO_MIP_STALL_GAIN": "3e-2w"},
}
