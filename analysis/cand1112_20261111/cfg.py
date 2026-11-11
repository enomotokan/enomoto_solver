"""束 cand1112 (2026-11-11)。cand1111 から、Mittelmann で害の出た CUTCONF・TREEFIX を外した 7 項目
(analysis/cand1111_20261111/README.md の切り分け)。既定 (def) は環境変数なし。"""
CAND1112 = {
    "ENOMOTO_MIP_ROOTSTOP": "b",
    "ENOMOTO_MIP_RCSTRONG": "irk",
    "ENOMOTO_MIP_LMIP_SWAP": "1",
    "ENOMOTO_MIP_SYMPRE": "probe",
    "ENOMOTO_MIP_PLSB": "gxu",
    "ENOMOTO_MIP_GAP1": "u",
    "ENOMOTO_MIP_GAP4": "rocn",
}
CFGS = {"def": {}, "cand1112": CAND1112}
