"""段ごとの表 (GAP2)。使い方: python table.py [名札 (既定 def)] — logs/ の ours_<名札>_s* と highs_h_s* を問題ごとに並べる。
ENOMOTO: 前段 (MIP の時計の前: 前処理・LP プロービング・プロービング・対称性など) / 根の LP / カット (選び方の試しを含む) / 発見法 (サブ MIP の待ちを含む) /
裏のカットの待ち / 再スタートの回数 / 木。区間 (再スタートで分かれる根) の分は足す。HiGHS: 前処理 / 根の終わり (前処理の後から) / 木 / サブ MIP の秒。"""
import glob, json, os, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
import stages
D = Path(__file__).parent / "logs"
tag = sys.argv[1] if len(sys.argv) > 1 else "def"
probs = ["drayage-100-23", "drayage-25-23", "p200x1188c", "neos8", "neos-827175", "pg", "swath1", "physiciansched6-2", "neos-960392", "cod105"]
if len(sys.argv) > 2:
    probs = sys.argv[2].split(",")
print("| 問題 | 種 | ENOMOTO 計 | 前段 (うち LP プロービング・プロービング・対称性) | 根の LP | カット (ラウンド) | 発見法 | 裏のカットの待ち | 根のその他 | 再スタート | 木 | HiGHS 計 | HiGHS 前処理 | HiGHS 根の終わり | HiGHS サブ MIP |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for p in probs:
    for s in (1, 2):
        f = D / f"{p}_ours_{tag}_s{s}.log"
        h = D / f"{p}_highs_h_s{s}.log"
        if not f.exists():
            continue
        r = stages.ours(f)
        sg = r["segs"]
        S = lambda k: sum(x[k] for x in sg)
        pre = r["pre"]
        hd = stages.highs(h) if h.exists() else {}
        tot = r["total"] or 0
        print(f"| {p} | {s} | {tot:.1f} | {r['premip']:.1f} ({pre.get('lpprobe',0):.1f}・{pre.get('probing',0)+pre.get('equiv',0):.1f}・{pre.get('symmetry',0):.1f}) | {S('rootlp'):.1f} | {S('cuts'):.1f} ({S('rounds')}) | {S('heur'):.1f} | {S('bgwait'):.1f} | {S('rest')+S('pre'):.1f} | {len(sg)-1} | {S('tree'):.1f} | "
              f"{hd.get('total',0):.1f} | {hd.get('presolve',0):.1f} | {(hd.get('rootend') or 0)-hd.get('presolve',0):.1f} | {hd.get('submip_solve',0):.1f} |")
