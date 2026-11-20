"""最終計測 (2026-11-18、利用者の依頼「mittelmann 全題と miplib で最終計測、SCIP と HiGHS 比で遅い問題を調べる」)。
def: 今の既定 4 スレッド。def_t1: 今の既定 1 スレッド (run.py の TH)。v019: PyPI の 0.1.9 の既定 4 スレッド (同じ機械で対にする)。
SCIP 10.0.2・HiGHS 1.15.1) は 2026-11-17 の行を使う (analysis/vsscip_20261117・analysis/vshighs_20261117、同じ種)。"""
CFGS = {"def": {}, "def_t1": {}, "v019": {}}
