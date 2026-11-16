"""誤答の切り分け W: M の unitcal_7 種 2 で cand1116 が 19642176.08 を「最適」(下界も同値) と返した (真の最適 19635558.2440195、最小化)。
python planW.py → shards_W.json。設定: cand1116・cand1117 と、cand1116 から鍵を 1 つずつ外した 7 通り (cfg.py の WRONG: w16_m_*) × 種 1〜8、
600 秒・4 スレッド (72 回)。unitcal_7 は 1 回 250〜550 秒なので 1 分担 4 回 (最大約 42 分)、18 分担。
回し方 python run.py W K (K = 0〜17)、集計 python summW.py > summ_W.txt (設定ごとの誤答の数: 「最適」で目的値か下界が真の最適値より 1e-6 (相対) 以上大きい)。
デバッグの検査: run.py の W では ENOMOTO_MIP_DEBUG_SOL に前処理の後の問題の最適解 (unitcal_7_pre.sol.gz、前処理は 9 設定とも同じ問題になることを確かめた。
HiGHS 1.15.1 mip_rel_gap=0 で 19635558.2440195) を渡し、ENOMOTO_MIP_LOG=1 の標準エラーから MIP_DEBUG_SOL の行と最後の 60 行を logs/W_設定_s種.log.gz に残す
(最適解を切った所 = 刈り込み・カット・伝播などが分かる。再起動の後は検査が外れる)。
4 スレッドで決定的でないので、同じ種でも再現しないことがある (cand1116 の 2026-11-15 の回の種 2 は正しかった)。"""
import json
from pathlib import Path
from cfg import WRONG
D = Path(__file__).resolve().parent
CF = ["cand1116", "cand1117"] + list(WRONG)
runs = [["unitcal_7", c, s, 600] for s in range(1, 9) for c in CF]
N = 18
shards = [runs[i::N] for i in range(N)]
(D / "shards_W.json").write_text(json.dumps({"shards": shards, "cfgs": CF}, indent=0))
print("shards_W.json:", N, "分担、回数", [len(s) for s in shards], "設定", CF)
