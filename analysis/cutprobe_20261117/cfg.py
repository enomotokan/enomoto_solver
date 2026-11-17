"""カットの選び方の下見の試作 `ENOMOTO_MIP_CUTSEL_PROBE_AFFORD` の対の比べ (treeclose 9.64 節、2026-11-17)。
def = 既定、aff = 1 ラウンド目の両側の時間から k ラウンドの比べが残りの予算に収まらないと見込めるときは 1 ラウンド目の値で決める。"""
CFGS = {"def": {}, "aff": {"ENOMOTO_MIP_CUTSEL_PROBE_AFFORD": "1"}}
