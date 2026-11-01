"""neos-960392 の HiGHS の前処理だけを log_dev_level=1 で行い、規則ごとの削減の数をログに出す。

使い方: python presolve_stats.py <ログ>
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
import highspy  # noqa: E402

mps = rb.fetch("neos-960392", Path(".miplib_cache"))
h = highspy.Highs()
h.setOptionValue("output_flag", True)
h.setOptionValue("log_to_console", False)
h.setOptionValue("log_file", sys.argv[1])
h.setOptionValue("log_dev_level", 1)
h.readModel(str(mps))
h.presolve()
