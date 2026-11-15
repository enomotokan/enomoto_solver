"""bgx_check1 の 1 回ごとの表 (logs/<設定>_<回>.log と res.jsonl から)。使い方: python summ.py [設定,...]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
res = {(r["cfg"], r["run"]): r for r in map(json.loads, open(D / "res.jsonl"))}
want = sys.argv[1].split(",") if len(sys.argv) > 1 else None
print("| 設定 | 回 | 結果 | 秒 | 主の根のカット (ラウンド・LP のカット・下界・道) | 写しの根の LP | 写しの各ラウンド (目的値 at 秒) | 止めの合図 (待ち・理由) | 写しの終わり | 写しの LP の失敗 |")
print("|---|---|---|---|---|---|---|---|---|---|")
for (c, i), r in sorted(res.items(), key=lambda kv: (kv[0][0], kv[0][1])):
    if want and c not in want:
        continue
    L = (D / "logs" / f"{c}_{i}.log").read_text().splitlines()
    # 主のループは最初の "cut loop done"、写しのループはその後 (写しのログは [snapshot] root LP の後)
    k_snap = next((k for k, l in enumerate(L) if "[snapshot] root LP" in l), len(L))
    main = [l for l in L[:k_snap] if re.search(r"cut round \d+: \d+ cuts \(of", l)]
    done = next((l for l in L[:k_snap] if "cut loop done" in l), "")
    m = re.search(r"(\d+) in LP.*-> ([\d.e+]+)", done)
    sel = next((l for l in L if "cut selection probe (" in l), "")
    path = re.search(r"-> (\w+)", sel).group(1) if sel else "?"
    snap = L[k_snap] if k_snap < len(L) else ""
    sm = re.search(r"\((\d+) iters, ([\d.]+)s\)", snap)
    fails = sum(1 for l in L[k_snap:] if "LP status" in l and "removed the cuts" in l)
    so = next((l for l in L if "rounds so far" in l), "")
    rounds = re.findall(r"([\d.]+) at ([\d.]+)s", so)
    wt = next((l for l in L if "background cuts: waited" in l), "")
    w = re.search(r"waited ([\d.]+)s for progress \(cap ([\d.]+)s, (\w+)\), (\d+) rounds.*\(([\d.]+)s\)", wt)
    fin = next((l for l in L if re.search(r"background cuts: \d+ rounds", l)), "")
    f = re.search(r"(\d+) rounds, (.*)", fin)
    finr = (f.group(1) + " ラウンド・" + ("使う" if "not used" not in fin else "不採用")) if f else "—"
    rs = ", ".join(f"{float(o):.0f}@{t}" for o, t in rounds) or "なし"
    print(f"| {c} | {i} | {r['status']} | {r['time']:.1f} | {len(main)}・{m.group(1) if m else '?'}・{float(m.group(2)) if m else 0:.0f}・{path} | "
          f"{sm.group(1) + ' 反復 ' + sm.group(2) + ' 秒' if sm else '?'} | {rs} | "
          f"{(w.group(5) + ' 秒に ' + w.group(1) + ' 秒 (' + w.group(3) + ')') if w else '—'} | {finr} | {fails} |")
