"""lpgap の P の比べの設定 (名前が _t1 で終われば 1 スレッド)。ths = 微小値の切り捨て (6 節)、t1i = 微小値の切り捨てに U 段の起点も (6.4 節)、rz = 破綻の後の解き直しで Zero 列も |alpha| 最大 (8 節)。"""
CFGS = {"def_t1": {}, "ths_t1": {"ENOMOTO_LU_TINY_HS": "1"}, "rz_t1": {"ENOMOTO_SIMPLEX_RETRY_ZMAX": "1"}, "t1i_t1": {"ENOMOTO_LU_TINY_HS": "1i"}}
