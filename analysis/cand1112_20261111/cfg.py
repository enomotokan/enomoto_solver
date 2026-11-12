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

# leave-one-out (2026-11-12): Mittelmann で 2 種とも遅い fiball・mc11・n2seq36q・swath1 の原因の項目を探す。
# "-X" は CAND1112 から X だけを外した形。
for _k in list(CAND1112):
    CFGS["-" + _k.replace("ENOMOTO_MIP_", "")] = {k: v for k, v in CAND1112.items() if k != _k}
