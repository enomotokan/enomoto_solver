"""batch1〜6.txt (neos859080・beasleyC3・n2seq36q を def・rob・both・mv・par、neos859080 の mvd、利用者の案 first3・par3s で) の集計
(python3 summ1.py > summ1.txt)。beasleyC3 の par (batch1) は確かめの解き直しの誤りで別の基底が作られず既定と同じだったので、直した後の
par2 (batch3) を par として数える。beasleyC3 の種 6 (batch1) はビルドと同時に走ったので、回し直した s6r (batch3) を使う。
試作の働き (2 回目のループ・写しの選び) と根の LP の勝ち負けはログ (runs/*.log) から。"""
import json, re, statistics
from pathlib import Path
D = Path(__file__).resolve().parent
rows = {}
lines = []
for b in ("batch1.txt", "batch2.txt", "batch3.txt", "batch4.txt", "batch5.txt", "batch6.txt", "batch7.txt", "batch8.txt", "batch9.txt", "batch10.txt", "batch11.txt", "batch12.txt"):
    if (D / b).exists():
        lines += open(D / b).read().splitlines()
for l in lines:
    if " " not in l.strip():
        continue
    n, j = l.split(" ", 1)
    try:
        r = json.loads(j)
    except Exception:
        continue
    p, c, s = n.rsplit("_", 2)
    if p == "beasleyC3" and (c == "par" or (s == "s6" and c in ("def", "rob", "both", "mv"))):
        continue
    r["_name"] = n
    if s == "s6r":
        s = "s6"
    if c == "par2":
        c = "par"
    rows.setdefault((p, c), []).append((s, r))
print("問題        設定   回  中央値   最悪    最良  時間切れ  根の下界 (中央値・最小・最大)  暫定解 (中央値)")
for (p, c), v in rows.items():
    ts = [r["time"] for _, r in v]
    rb = [r["root_bound"] for _, r in v if r.get("root_bound") is not None]
    obj = [r["objective"] for _, r in v if r.get("objective") is not None]
    print(f"{p:11s} {c:5s} {len(v):2d} {statistics.median(ts):7.1f} {max(ts):7.1f} {min(ts):6.1f} {sum(r['status']=='time_limit' for _, r in v):3d}"
          f"   {statistics.median(rb):.4f} {min(rb):.4f} {max(rb):.4f}   {statistics.median(obj) if obj else '-'}")
print()
for (p, c), v in rows.items():
    for s, r in v:
        log = D / "runs" / f"{r['_name']}.log"
        t = log.read_text(errors="replace") if log.exists() else ""
        ex = []
        if "robust root IPM: second cut loop" in t:
            ex.append("rob 2 回目")
        elif "robust root IPM" in t:
            ex.append("rob 戻す")
        m = re.search(r"root LP both: probe gain .* -> (.*)", t)
        if m:
            ex.append("both " + m.group(1))
        k = len(re.findall(r"optface move: probe at the moved vertex", t))
        if "optface move: second cut loop" in t:
            ex.append(f"mv 2 回目 (試し {k})")
        elif k:
            ex.append(f"mv 戻す (試し {k})")
        m = re.search(r"optface first: (\d+) cuts .*obj \S+ -> (\S+)", t)
        if m:
            ex.append(f"first {m.group(1)} 本 -> {float(m.group(2)):.4g}")
        m = re.search(r"optface parcuts: use copy (\d+)", t)
        if m:
            ex.append("par 写し " + m.group(1))
        m = re.search(r"optface parcuts: (\d+) background copies \((\d+) from", t)
        if m and not ex:
            ex.append(f"par 写し {m.group(1)} 本 (別の基底 {m.group(2)})")
        m = re.findall(r"background cuts: \d+ rounds, \d+ cut rows \(was \d+\), obj (\S+) -> (\S+)", t)
        bg = f" 裏 {float(m[-1][0]):.4g}->{float(m[-1][1]):.4g}" if m else ""
        re_end = re.findall(r"after root heuristics: .*\(([\d.]+)s\)", t)
        print(f"{p:11s} {c:5s} {s:3s} {r['status']:10s} {r['time']:7.1f} 根 {r['root_bound']:.4f} 解 {r.get('objective')} 節 {r['nodes']}"
              f" 根の終わり {re_end[0] if re_end else '-'}{bg} {'; '.join(ex)}")
