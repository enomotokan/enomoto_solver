"""ログ (logs/*_<tag>_<k>.log、各行の頭は起動からの経過秒) から、節目の時刻を求解の始めからの秒にして並べる。
使い方: python breakdown.py <tag>。求解の始め = 「solve returned ... after Xs」の行の時刻 - X。"""
import re, sys, glob, statistics
from pathlib import Path
D = Path(__file__).resolve().parent
KEYS = [("前処理 (最初)", r"MIP: presolve: rows"), ("LP による固定", r"MIP: LP probing: root LP"), ("プロービング", r"MIP: probing: \d+ of"),
        ("クリーク行の拡張", r"MIP: clique extension:"), ("対称性", r"MIP: symmetry: \d+ generators"), ("根の LP", r"MIP: root LP [-0-9]"),
        ("カットのループ (1 回目)", r"MIP: cut loop done"), ("根のヒューリスティクス (1 回目)", r"MIP: after root heuristics"),
        ("FJ の待ち", r"FJ thread joined"), ("木の探索の始め", r"tree search with"), ("最後の暫定解", r"new incumbent"), ("終わり", r"solve returned")]
tag = sys.argv[1]
for p in ["drayage-100-23", "drayage-25-23", "neos8", "air03"]:
    print(f"== {p} ({tag})")
    for f in sorted(glob.glob(str(D / "logs" / f"{p}_{tag}_*.log"))):
        L = Path(f).read_text().splitlines()
        st = lambda l: float(l[1:8])
        end = [l for l in L if "solve returned" in l][0]
        t0 = st(end) - float(re.search(r"after ([0-9.]+)s", end).group(1))
        row = []
        for name, pat in KEYS:
            hit = [l for l in L if l.startswith("[") and re.search(pat, l)]
            if not hit: continue
            h = hit[-1] if name in ("最後の暫定解", "終わり") else hit[0]
            row.append(f"{name}={st(h) - t0:.2f}")
        rs = sum("restart" in l and re.search(r"restart \d \(", l) is not None for l in L)
        print(f"  {Path(f).name}: restarts={rs} " + " ".join(row))
