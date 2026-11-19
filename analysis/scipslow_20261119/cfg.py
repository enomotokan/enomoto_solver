"""scipslow の設定: def = 今の既定、pu50 = プロービングを、固定も締め付けも増えない試しが 50 回続いたら打ち切る (README 3 節)。"""
CFGS = {
    "def": {},
    "pu50": {"ENOMOTO_MIP_PROBE_USELESS": "50"},
}
