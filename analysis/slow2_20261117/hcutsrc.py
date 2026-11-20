"""計装した HiGHS 1.15.1 (ログだけのパッチ hcutsrc.patch、HighsSeparation・HighsCutPool・HighsLpRelaxation) で根まで解き、
根のラウンドごとのカットの出どころ (implied bound・クリーク・表・経路・mod-k) を数える。
python hcutsrc.py <計装版 highspy を入れた置き場> <問題> [秒]  → logs/hcutsrc_<問題>.log
4 スレッド・mip_max_nodes=1・mip_rel_gap 0。CUTSRC の行: 1 ラウンドの前後の目的値、作ったカットの数 (gen)、LP の中のカット行の
本数/双対が 0 でない本数/双対 x 行の値の和 (出どころごと)、モデルの行の双対 x 行の値の和。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
tgt, p = sys.argv[1], sys.argv[2]; tl = float(sys.argv[3]) if len(sys.argv) > 3 else 600.0
code = f"""
import sys; sys.path.insert(0, {tgt!r})
import highspy
h = highspy.Highs()
h.setOptionValue("threads", 4); h.setOptionValue("time_limit", {tl}); h.setOptionValue("mip_max_nodes", 1); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel({str(WT / ".miplib_cache" / (p + ".mps"))!r}); h.run()
"""
env = dict(os.environ, HIGHS_CUTSRC_LOG="1")
with open(D / "logs" / f"hcutsrc_{p}.log", "w") as f:
    subprocess.run([sys.executable, "-I", "-c", code], env=env, stdout=f, stderr=subprocess.STDOUT)
