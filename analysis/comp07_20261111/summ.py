"""COMP07: logs/ のログと res.jsonl から、回ごとに 結果・根のカットのループの終わり (時刻・下界)・暫定解の推移 (値@時刻[出どころ])・
再スタートの回数を 1 行にまとめる。時刻はログの各求解の中の経過秒 (再スタートで 0 に戻る) なので、再スタートごとに区切る。
使い方: python analysis/comp07_20261111/summ.py [絞り込みの文字列 ...]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / "res.jsonl").read_text().splitlines()]
flt = sys.argv[1:]
for r in rows:
    tag = f"{r['problem']}_{r['build']}_{r['cfg']}_s{r['seed']}_{r['tl']}"
    if flt and not all(f in tag for f in flt):
        continue
    log = D / "logs" / f"{tag}.log"
    res = r["result"]
    segs, cur = [], {"inc": [], "cut": None, "heur": None}
    last_t = 0.0
    if log.exists():
        for l in log.read_text(errors="replace").splitlines():
            if re.match(r"MIP: restart \d+ \(", l):
                segs.append(cur); cur = {"inc": [], "cut": None, "heur": None}
            m = re.match(r"MIP: new incumbent (\S+) at node (\d+) \((\S+)s\) \[(.*)\]", l)
            if m:
                cur["inc"].append(f"{float(m[1]):g}@{float(m[3]):.0f}{'' if m[2] == '0' else 'n' + m[2]}[{m[4][:6]}]")
            m = re.match(r"MIP: cut loop done: .* -> (\S+)", l)
            if m and cur["cut"] is None:
                cur["cut"] = float(m[1])
                cur["cut_t"] = last_t
            m = re.match(r"MIP: after root heuristics: incumbent (.*) \((\S+)s\)", l)
            if m and cur["heur"] is None:
                cur["heur"] = float(m[2])
            m = re.search(r"obj \S+ \((\d+\.\d+)s\)\s*$", l)
            if m and l.startswith("MIP: cut round"):
                last_t = float(m[1])
    segs.append(cur)
    st = res.get("status"); t = res.get("time") or 0
    print(f"{r['build']:>5} {r['cfg']:<12} s{r['seed']} {st[:9]:<9} {res.get('objective')!s:>5} {t:6.1f}s nodes {res.get('nodes')!s:>6} rst {len(segs) - 1}")
    for i, s in enumerate(segs):
        cut = f"cut {s['cut']:.3f}@{s.get('cut_t', 0):.0f}" if s["cut"] is not None else "cut -"
        print(f"      [{i}] {cut} heur_end {s['heur']} inc {' '.join(s['inc'][:12])}")
