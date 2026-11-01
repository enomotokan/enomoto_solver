"""ログのまとめ: 問題 × 設定 × 回の時間・状態・目的値と、外側 (深さ 1) の RENS (more nodes) のやり直しの本数・秒・結果、省いた本数。
使い方: python summ.py [ログのディレクトリ (既定 logs)]。表は標準出力 (summ.txt に保存する)。"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
L = D / (sys.argv[1] if len(sys.argv) > 1 else "logs")
HEUR = re.compile(r"^MIP: root heuristic RENS \(more nodes\): (.*?) \(([\d.]+)s\)")
SKIP = re.compile(r"^MIP:   RENS \(more nodes\) skipped")
rows = defaultdict(dict)
for f in sorted(L.glob("*.log")):
    m = re.match(r"(.+)_([a-z0-9_]+)_(\d+)\.log$", f.name)
    # 設定名に _ を含むので、問題名は既知の設定名から切る
    stem = f.name[:-4]
    prob, rest = None, None
    for c in ["real_s0", "real_s2", "def_s0", "def", "real"]:
        mm = re.match(rf"(.+)_{c}_(\d+)$", stem)
        if mm:
            prob, cfg, rep = mm[1], c, int(mm[2])
            break
    if prob is None:
        continue
    res = None
    retries, skips, outer_more = [], 0, False
    for line in f.read_text(errors="replace").splitlines():
        if line.startswith("{") and '"status"' in line:
            try:
                res = json.loads(line)
            except Exception:
                pass
        mm = HEUR.match(line)
        if mm:
            retries.append((mm[1], float(mm[2])))
        if line.startswith("MIP: root heuristic RENS (more nodes)") or "heuristic RENS (more nodes)" in line:
            pass
        if SKIP.match(line):
            skips += 1
    rows[(prob, cfg)][rep] = (res, retries, skips)
probs = sorted({k[0] for k in rows})
cfgs = [c for c in ["def", "real", "real_s0", "real_s2", "def_s0"] if any(k[1] == c for k in rows)]
print("問題 | 設定 | 回ごとの 秒 (目的値) [やり直し: 結果/秒 ; 省いた本数]")
for p in probs:
    for c in cfgs:
        if (p, c) not in rows:
            continue
        parts = []
        for rep in sorted(rows[(p, c)]):
            res, rt, sk = rows[(p, c)][rep]
            if res is None:
                parts.append("NO RESULT")
                continue
            st = "" if res["status"] == "optimal" else "T"
            t = f"{res['time']:.1f}{st}"
            obj = res.get("objective")
            r = ",".join(f"{a[:20]}/{b:.1f}s" for a, b in rt) or "-"
            parts.append(f"{t} ({obj:.6g}) [{r} ; skip {sk}]" if obj is not None else f"{t} (none) [{r} ; skip {sk}]")
        print(f"{p} | {c} | " + " | ".join(parts))
