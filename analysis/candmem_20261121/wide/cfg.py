# candmem の広い計測の設定 (analysis/candmem_20261121/README.md)。new は既定 (tableau 行の集約行を作りながら渡す)、
# old は ENOMOTO_MIP_SEP_TAB_STREAM=0 で以前の形 (集約行を全部作ってから渡す)。候補は同じで、時間とメモリ (max_rss_mb) だけを比べる
CFGS = {"old": {"ENOMOTO_MIP_SEP_TAB_STREAM": "0"}, "new": {}}
