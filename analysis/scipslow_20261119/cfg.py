"""scipslow の設定: def = 今の既定、pu50 = プロービングの打ち切り (README 3・5 節)、rep = ORBPP_REPROBE (README 4・6 節)。"""
CFGS = {
    "def": {},
    "pu50": {"ENOMOTO_MIP_PROBE_USELESS": "50"},  # 2 版目 (新しい含意も数える。1 版目の結果は res_P_v1_s*.jsonl)
    # graph20-20-1rand 対策 (README 4・6 節): 静的なオービトープの固定の後に伝播・オービトープの固定・プロービングを繰り返し、冗長な行を外す
    "rep": {"ENOMOTO_MIP_ORBPP_REPROBE": "1"},
}
