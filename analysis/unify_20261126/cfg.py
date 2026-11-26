"""unify の写し違いの直し `ENOMOTO_MIP_UNIFY_FIX=1` (analysis/unify_20261126/README.md の 4 節) の P での比べの設定。既定 (def_t1・def_t4) と
直し (uf_t1・uf_t4) を 1・4 スレッドで。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。
形は analysis/polishp_20261125/cfg.py と同じ (summ.py・plan.py・run.py をそのまま使う)。"""
P1 = {  # 番号: (名前, 環境変数, 1 スレッドで回すか, 4 スレッドで回すか, 記録)
    1: ("uf", {"ENOMOTO_MIP_UNIFY_FIX": "1"}, True, True, "unify_20261126"),
}
CFGS = {"def_t1": {}, "def_t4": {}}
for k, (name, env, t1, t4, _) in P1.items():
    if t1: CFGS[f"{name}_t1"] = env
    if t4: CFGS[f"{name}_t4"] = env
