"""inhpool の広い計測の設定 (analysis/inhpool_20261121)。def: 今の既定 (引き継ぎの鎖を共有の待ち行列でも共有する、analysis/biglp_20261121 の 8 節)。
old: `ENOMOTO_MIP_INHPOOL=0` (従来どおり、共有の待ち行列に出すノードで鎖を平らに展開する)。どれも 4 スレッド (run.py の TH)。"""
CFGS = {"def": {}, "old": {"ENOMOTO_MIP_INHPOOL": "0"}}
