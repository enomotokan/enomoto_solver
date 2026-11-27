"""既定候補を全部入れた組 (combo) と既定の比べ (2026-11-24、利用者の依頼「既定候補を全部入れて、Mittelmann 186 問と MIPLIB 240 問を計測」、README.md)。
名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。"""
COMBO = {
    "ENOMOTO_MIP_SB_PROBEBD": "1",             # sbpb (treegap_20261123、protoscreen 11)
    "ENOMOTO_MIP_CUTOFF_USER_TREE": "1",       # cutu (ti6_20261124、protoscreen 12)
    "ENOMOTO_MIP_NODELP_BFRT_GROUP": "1",      # bfrt (lpiter_20261123、protoscreen 9)
    "ENOMOTO_MIP_PROOFPROP": "i",              # ppi (proofprop_20261123、protoscreen 10。2 スレッド以上でだけ働く)
    "ENOMOTO_MIP_HEUR_ADAPT": "1",             # heuradapt_20261124 (HEUR1T の f・l、FP_STAGE3・POOR_NOINC・P2RIT=a・FJSYNC_LABEL を中で割合つきで使う)
    "ENOMOTO_MIP_NODEROWS_ADAPT": "t",         # nodespeed_20261123 9.6 節
    "ENOMOTO_MIP_ENUM_ADAPT": "1",             # cutgap_20261123 8 節 (ENUM_PRESOLVE=all・ENUM_WHERE=loop・PRR_TIGHT を中で使う)
    "ENOMOTO_MIP_CUTSEL_PROBE_TIE_FEWER": "1",  # shelved_20261122・cutrobust_20261124
}
CFGS = {"def_t1": {}, "def_t4": {}, "combo_t1": dict(COMBO), "combo_t4": dict(COMBO)}
