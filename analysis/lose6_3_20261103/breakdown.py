"""lose6_3: ENOMOTO のログ (run.py の、行頭に起動からの経過秒) から段 (再スタートで区切った根の処理) ごとの時刻を拾う。
秒は求解の始め (「solve returned ... after Xs」の行の時刻 - X) から。使い方: python breakdown.py logs/<問題>_<設定>_<回>.log ...
各段: 始め・根の LP の終わり (値)・主のカットのループの終わり (ラウンド数・値)・根のヒューリスティクスの終わり・並列サブ MIP の待ち・
裏のカットの待ち・段の終わり。暫定解の時刻と出どころ。木の時間。"""
import gzip, re, sys

def rd(f):
    return (gzip.open(f, "rt") if f.endswith(".gz") else open(f)).read().splitlines()

def parse(f):
    L = rd(f)
    T = lambda l: float(l[1:8])
    m = [(T(l), float(re.search(r"after ([\d.]+)s", l).group(1))) for l in L if "solve returned" in l]
    t0 = m[-1][0] - m[-1][1] if m else 0.0
    st = [{"start": 0.0, "inc": []}]
    out = {"t0": t0, "stages": st, "tree": None}
    cur = st[0]
    for l in L:
        if not l.startswith("["):
            continue
        t = T(l) - t0; s = l[10:]
        if re.match(r"MIP: restart \d+ \(", s):
            cur["end"] = t; cur["end_kind"] = "restart"
            cur = {"start": t, "inc": []}; st.append(cur)
        elif s.startswith("MIP: root LP ") and "lp0" not in cur and re.match(r"MIP: root LP [-\d.e+]+ \(", s):
            cur["lp0"] = t; cur["lp0_val"] = float(s.split()[3])
        elif s.startswith("MIP: cut loop done") and "cut" not in cur:
            cur["cut"] = t; cur["cut_val"] = float(s.rsplit("-> ", 1)[1])
            cur["rounds"] = cur.get("r", 0)
        elif re.match(r"MIP: cut round (\d+): \d+ cuts", s) and "cut" not in cur:
            cur["r"] = int(re.match(r"MIP: cut round (\d+)", s).group(1)) + 1
        elif "cut selection probe (" in s and "cut" not in cur:
            cur["probe"] = cur.get("probe", 0) + float(re.search(r"probe ([\d.]+)s in total", s).group(1))
        elif s.startswith("MIP: after root heuristics"):
            cur["heur"] = t
        elif "parallel sub-MIP (" in s and "status" in s:
            cur.setdefault("sub", []).append((t, re.search(r"\((.*?)\)", s).group(1), re.search(r"status (\w+)", s).group(1), float(re.search(r"([\d.]+)s$", s).group(1))))
        elif "background cuts: determ: waited" in s:
            cur["bgwait"] = float(re.search(r"waited ([\d.]+)s", s).group(1))
            m2 = re.search(r"(\d+) rounds", s); cur["bgrounds"] = int(m2.group(1))
        elif re.match(r"MIP: background cuts: \d+ rounds, ", s):
            m2 = re.search(r"-> (\S+) \(", s); cur["bg_val"] = float(m2.group(1)) if m2 else None
        elif s.startswith("MIP: new incumbent"):
            cur["inc"].append((round(t, 2), float(s.split()[3]), s.rsplit("[", 1)[1].rstrip("]")))
        elif s.startswith("MIP: tree search with"):
            out["tree"] = t
        elif s.startswith("MIP: solve returned"):
            out["end"] = t
        elif "MIP: solving the root LP" in s and "pre" not in out:
            out["pre"] = t
    cur.setdefault("end", out.get("end")); cur.setdefault("end_kind", "tree" if out["tree"] else "end")
    return out

def fmt(o):
    rows = []
    for k, s in enumerate(o["stages"]):
        sub = s.get("sub", [])
        subs = ", ".join(f"{n} {st[:4]} {d:.2f}" for _, n, st, d in sub if d >= 0.05)
        rows.append(f"| {k} | {s['start']:.2f} | {s.get('lp0', 0):.2f} ({s.get('lp0_val', float('nan')):.6g}) | {s.get('cut', 0) - s.get('lp0', 0):.2f} ({s.get('rounds', '?')} 回, 試し {s.get('probe', 0):.2f}) → {s.get('cut_val', float('nan')):.7g} | "
                    f"{s.get('heur', 0) - s.get('cut', 0):.2f} | {subs} | {s.get('bgwait', 0):.2f} ({s.get('bgrounds', '-')} 回 → {s.get('bg_val') or '-'}) | {s['end']:.2f} {s['end_kind']} | "
                    + "; ".join(f"{z:.7g} {w} ({t:.2f})" for t, z, w in s["inc"]) + " |")
    return rows

if __name__ == "__main__":
    for f in sys.argv[1:]:
        o = parse(f)
        print(f"\n{f}: 計 {o.get('end', 0):.2f} 秒、根の LP を解き始めるまで (前処理など) {o.get('pre', 0):.2f}、木 {(o['end'] - o['tree']) if o['tree'] else 0:.2f}")
        print("| 段 | 始め | 根の LP | カット (ラウンド) → 下界 | ヒューリ | 並列サブ MIP (0.05 秒以上) | 裏待ち (ラウンド → 値) | 終わり | 暫定解 |")
        print("|---|---|---|---|---|---|---|---|---|")
        print("\n".join(fmt(o)))
