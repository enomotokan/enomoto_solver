"""組み直した組み合わせの第 1 段 (P、analysis/combo2_20261125) の設定。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。
どれも既定 (6762d9c3c 以降: POLISH_STRICT=2 + POLISH_WARM=d が既定) に足す。既定と試作は同じ版で回す。

環境変数の名前と値は、先端 a5008f9d8 のコードで確かめた (README 1 節に行番号):
- NODEROWS_ADAPT=r0k23: src/mip/solver.rs `nra_on` (値があれば有効) と `nra_has` (文字を含むか)。r・0・k・2・3 がどれも効く。
  r0k23 は comboharm 10.9 節で確かめた形そのまま。
- ENUM_ADAPT=1s: src/mip/mod.rs `enum_adapt_on` (値が 0 でなければ有効) と `s` の判定。
- HEUR_ADAPT=1mk: src/mip/solver.rs `heur_adapt` (0 でなければ有効) と `heur_adapt_flag` (文字を含むか)。
  1 の後に m と k を並べれば、両方とも入る。
- FP_STAGE3_LS=a: src/mip/heuristics.rs (0 でなければ有効、`b` 以外は FJ2・Local-MIP だけ)。
  第 3 段は HEUR_ADAPT が開くので、HEUR_ADAPT と組んで初めて効く。
- LMIP_SCORE_TICKS=1、NODELP_BFRT_GROUP=1、CUTSEL_PROBE_TIE_FEWER=1、CUTOFF_USER_TREE=1: どれも 0 でなければ有効。
  NODELP_BFRT_GROUP は `a` 以外の値で同じ形。
- PROOFPROP=i: src/mip/solver.rs `proofprop` (文字を含むか)。木を分け合うときだけ効くので、1 スレッドでは働かない。
- SB_ADAPT=1 (c2b だけ): src/mip/solver.rs `sb_adapt_on` (0 でなければ有効)。
  強分枝の子の伝播は、SB_PROBEBD を置かなければ、`sbad_f` の f_B の確率でノードごとに使う (SB_PROBEBD があれば常に使う)。
  そのため c2b では SB_PROBEBD を置かない (適応形として働く)。主の木だけで効き、サブ MIP では使わない。"""
C2 = {
    "ENOMOTO_MIP_NODEROWS_ADAPT": "r0k23",
    "ENOMOTO_MIP_ENUM_ADAPT": "1s",
    "ENOMOTO_MIP_HEUR_ADAPT": "1mk",
    "ENOMOTO_MIP_FP_STAGE3_LS": "a",
    "ENOMOTO_MIP_LMIP_SCORE_TICKS": "1",
    "ENOMOTO_MIP_NODELP_BFRT_GROUP": "1",
    "ENOMOTO_MIP_CUTSEL_PROBE_TIE_FEWER": "1",
    "ENOMOTO_MIP_CUTOFF_USER_TREE": "1",
    "ENOMOTO_MIP_PROOFPROP": "i",
}
C2A = dict(C2)
C2B = dict(C2, ENOMOTO_MIP_SB_ADAPT="1")
P1 = {1: ("c2a", C2A, True, True, "combo2_20261125"), 2: ("c2b", C2B, True, True, "combo2_20261125")}
CFGS = {"def_t1": {}, "def_t4": {}}
for k, (name, env, t1, t4, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
