"""unify の写し違いの直し (analysis/unify_20261126/README.md の 4 節・5 節) の P での比べの設定。名前が _t1 で終われば 1 スレッド、_t4 なら
4 スレッド (run.py の TH)。形は analysis/polishp_20261125/cfg.py と同じ (summ.py・plan.py・run.py をそのまま使う)。
- 第 1 弾 (4 節): 既定 (def) と `ENOMOTO_MIP_UNIFY_FIX=1` (uf)。shards.json・res_sK.jsonl。2026-11-26 に既定にした。
- 第 2 弾 (5 節): 既定 (def、第 1 弾を含む) と `=1zp` (uf2)。shards2.json・res2_sK.jsonl。集計は `UNIFY_SET=2 python summ.py 'res2_s*.jsonl'`。
CFGS は両方の名前を持つ (run.py はどの分担表でも回せる)。P1 と plan.py の設定 (PLAN) は環境変数 UNIFY_SET で選ぶ。"""
import os as _os
CFGS = {"def_t1": {}, "def_t4": {},
        "uf_t1": {"ENOMOTO_MIP_UNIFY_FIX": "1"}, "uf_t4": {"ENOMOTO_MIP_UNIFY_FIX": "1"},
        "uf2_t1": {"ENOMOTO_MIP_UNIFY_FIX": "1zp"}, "uf2_t4": {"ENOMOTO_MIP_UNIFY_FIX": "1zp"}}
if _os.environ.get("UNIFY_SET") == "2":
    P1 = {1: ("uf2", CFGS["uf2_t1"], True, True, "unify_20261126")}
    PLAN = ["def_t1", "def_t4", "uf2_t1", "uf2_t4"]
else:
    P1 = {1: ("uf", CFGS["uf_t1"], True, True, "unify_20261126")}
    PLAN = ["def_t1", "def_t4", "uf_t1", "uf_t4"]
