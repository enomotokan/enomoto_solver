"""ENOMOTO_MIP_LMIP_SCORE_TICKS (analysis/comboharm_20261124/app1-1.md) の P での比べ (2026-11-25、親の依頼) の設定。既定 (def_t1) と
SCORE_TICKS=1 (sct_t1) を 1 スレッドで (手間の期限の数え違いが効くのは 1 スレッドの早めの FJ/Local-MIP なので)。
形は analysis/protoscreen_20261124/cfg.py と同じ (summ.py をそのまま使う)。"""
P1 = {  # 番号: (名前, 環境変数, 1 スレッドで回すか, 4 スレッドで回すか, 記録)
    1: ("sct", {"ENOMOTO_MIP_LMIP_SCORE_TICKS": "1"}, True, False, "lmipticks_20261125"),
}
CFGS = {"def_t1": {}}
for k, (name, env, t1, t4, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
