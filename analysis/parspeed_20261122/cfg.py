"""parspeed の設定 (analysis/parspeed_20261122/README.md)。どれも今の既定 (環境変数なし) で、スレッド数と打ち切り値だけ変える。
t1・t4: 1・4 スレッド。t1c・t4c: さらに ENOMOTO_MIP_CUTOFF = 既知の最適値 - 1e-6 max(1, |最適値|) (run.py が問題ごとに入れる)。
打ち切り値を最適値の少し下にするのは、最適値の暫定解を持って始めたのと同じ刈り方にするため (暫定解 z の刈りの上限は
z - 1e-9 max(1,|z|) か目的の刻みの格子の点。打ち切り値より悪い解も暫定解としては受け取る: misc07 で確かめた)。"""
CFGS = {"t1": {}, "t4": {}, "t1c": {}, "t4c": {}}
def threads(cfg): return "1" if cfg.startswith("t1") else "4"
def has_cutoff(cfg): return cfg.endswith("c")
