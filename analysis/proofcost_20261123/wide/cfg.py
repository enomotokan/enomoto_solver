"""proofcost の広い計測の設定 (analysis/proofcost_20261123)。どちらも 1 スレッド (run.py の TH: 名前が _t1 で終われば 1 スレッド)。
pa は木を分け合う (2 スレッド以上) ときは既定に入っている仕組みなので、4 スレッドでは差が出ない。
def_t1: 今の既定。pa_t1: 試作 `ENOMOTO_MIP_PROOFCOST=pa` (証明の本数の上限 max(200, 4 × 列数) と、使われた印による年齢を 1 スレッドにも)。"""
CFGS = {"def_t1": {}, "pa_t1": {"ENOMOTO_MIP_PROOFCOST": "pa"}}
