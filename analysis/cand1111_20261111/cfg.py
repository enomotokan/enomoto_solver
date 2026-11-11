"""束 cand1111 (2026-11-11) の設定。取り込みの方針 (docs/backlog.md 冒頭) の基準を満たした試作を全部入れる。
既定 (def) は環境変数なし (cand1108zd + DRCR + SYMDYN=f)。"""
CAND1111 = {
    "ENOMOTO_MIP_ROOTSTOP": "b",        # ROOTSTOP: 再スタートが見込まれる根で裏のカットを起こさない (analysis/rootstop_20261111.md)
    "ENOMOTO_MIP_RCSTRONG": "irk",      # RCSTRONG: 暗黙の整数の切り捨てを根だけ (ir) + 被約費用の行の伝播 (k) (analysis/rcstrong_20261111.md)
    "ENOMOTO_MIP_LMIP_SWAP": "1",       # BINHEUR: 集合分割の行の中の入れ替えの手 (analysis/binheur_20261111.md)
    "ENOMOTO_MIP_TREEFIX": "pcu",       # TREEFIX: 木の中で暫定解が良くなったときの大域的な伝播など (analysis/treefix_20261111.md)
    "ENOMOTO_MIP_SYMPRE": "probe",      # SASSY: 対称性の検出の探り (analysis/sassy_20261110.md)
    "ENOMOTO_MIP_CUTCONF": "1",         # PAPER1: カットに基づく衝突解析 (analysis/cutconf_20261110.md)
    "ENOMOTO_MIP_PLSB": "gxu",          # PAPER2: 強分岐の打ち切り (analysis/plsb_20261110.md)
    "ENOMOTO_MIP_GAP1": "u",            # GAP1: 打ち切り値つきの根の LP プロービング (analysis/drcr2_20261111.md の fu)
    "ENOMOTO_MIP_GAP4": "rocn",         # GAP4: メモリの削減 (analysis/gap4mem_20261111.md)
}
CFGS = {"def": {}, "cand1111": CAND1111}
