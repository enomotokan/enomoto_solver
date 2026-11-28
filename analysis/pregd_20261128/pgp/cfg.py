"""PREGD (analysis/pregd_20261128) の P: 既定 (def_t1・def_t4) と試作 `ENOMOTO_MIP_PREGD=pg` (同じ列の固定と、支配列の固定を有界な一般の列に広げる形)
を 1・4 スレッドで (2026-11-28、親の依頼)。形は analysis/rootbound_20261126/prrto/cfg.py と同じ (summ.py をそのまま使う)。"""
P1 = {  # 番号: (名前, 環境変数, 1 スレッドで回すか, 4 スレッドで回すか, 記録)
    1: ("pg", {"ENOMOTO_MIP_PREGD": "pg"}, True, True, "pregd_20261128"),
}
CFGS = {"def_t1": {}, "def_t4": {}, "pg_t1": {"ENOMOTO_MIP_PREGD": "pg"}, "pg_t4": {"ENOMOTO_MIP_PREGD": "pg"}}
