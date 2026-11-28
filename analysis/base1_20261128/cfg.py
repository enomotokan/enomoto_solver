"""c2c の土台の部品を 1 つずつ既定に足して測る (analysis/base1_20261128、2026-11-28、親の依頼・利用者の承認)。
名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。どれも今の既定 (先端) に足す。部品の形は c3 (analysis/combo3_20261126/cfg.py) のもの。
- heur  : HEUR_ADAPT=1mkr (`r` は analysis/combo3_20261126 8.3 節の漏れの直し、analysis/heurfix_20261128)
- heurfp: HEUR_ADAPT=1mkr + FP_STAGE3_LS=a (FP_STAGE3_LS は HEUR_ADAPT が開く FP の第 3 段にだけ効くので、単独では働かない)
- nra   : NODEROWS_ADAPT=r0k23p + NRA_SHARE=1 (share は 4 スレッドだけ働く)
- enum  : ENUM_ADAPT=1st
- sbpb  : SB_PROBEBD=1 + SB_PROP_NO_LP=1 (c3 の nolp の形)"""
PARTS = {
    "heur": {"ENOMOTO_MIP_HEUR_ADAPT": "1mkr"},
    "heurfp": {"ENOMOTO_MIP_HEUR_ADAPT": "1mkr", "ENOMOTO_MIP_FP_STAGE3_LS": "a"},
    "nra": {"ENOMOTO_MIP_NODEROWS_ADAPT": "r0k23p", "ENOMOTO_MIP_NRA_SHARE": "1"},
    "enum": {"ENOMOTO_MIP_ENUM_ADAPT": "1st"},
    "sbpb": {"ENOMOTO_MIP_SB_PROBEBD": "1", "ENOMOTO_MIP_SB_PROP_NO_LP": "1"},
}
P1 = {i + 1: (n, e, True, True, "base1_20261128") for i, (n, e) in enumerate(PARTS.items())}
CFGS = {"def_t1": {}, "def_t4": {}}
for n, e in PARTS.items():
    CFGS[f"{n}_t1"] = dict(e); CFGS[f"{n}_t4"] = dict(e)
