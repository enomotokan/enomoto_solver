# ORBPP cond の広い計測の設定 (analysis/orbpp_20261121/README.md)。def は既定、cond は ENOMOTO_MIP_ORBPP=cond
# (列が MIN_COLS 未満のパッキング・分割のオービトープを、静的固定 + REPROBE で新しい固定が出るときだけ静的に扱う)
CFGS = {"def": {}, "cond": {"ENOMOTO_MIP_ORBPP": "cond"}}
