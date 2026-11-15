"""ENOMOTO のログ (ENOMOTO_MIP_LOG=1) から、再スタートで区切った段ごとの時刻 (段の中の経過秒) を拾って表にする。
使い方: python timeline.py logs/<問題>_<設定>_<k>.log ..."""
import gzip, re, sys
T = re.compile(r"\((\d+\.\d+)s\)\s*$")
def t_of(l):
    m = T.search(l); return float(m.group(1)) if m else None
def parse(path):
    lines = (gzip.open(path, "rt") if str(path).endswith(".gz") else open(path)).read().splitlines()
    phases, cur = [], None
    def new():
        return {"rootlp": None, "rounds": 0, "cutend": None, "heur_end": None, "bgwait": 0.0, "bgend": None, "end": None, "end_kind": None,
                "inc": [], "heur": [], "submip": [], "lb0": None, "lb1": None, "lpsolve": None}
    cur = new(); total = None
    for l in lines:
        t = t_of(l)
        if re.match(r"MIP: root LP [-0-9.e]+ \(", l) and cur["rootlp"] is None:
            m = re.search(r"root LP (\S+) \((\d+) iters, (\S+)s\)", l); cur["rootlp"] = float(m.group(3)); cur["lb0"] = float(m.group(1))
        elif re.match(r"MIP: cut round \d+: \d+ cuts", l):
            cur["rounds"] += 1; cur["cutend"] = t
        elif l.startswith("MIP: cut loop done"):
            cur["lb1"] = float(l.split("->")[-1].strip())
        elif l.startswith("MIP: new incumbent"):
            m = re.search(r"incumbent (\S+) at node (\d+) \((\S+)s\) \[(.*)\]", l); cur["inc"].append((float(m.group(3)), float(m.group(1)), m.group(4), int(m.group(2))))
        elif l.startswith("MIP: root heuristic "):
            m = re.match(r"MIP: root heuristic (.*): (\w+).*?\((\S+)s\)", l)
            if m and m.group(2) not in ("skipped",): cur["heur"].append((m.group(1), m.group(2), float(m.group(3))))
        elif "parallel sub-MIP (" in l and "status" in l or l.startswith("MIP:   sub-MIP:"):
            cur["submip"].append(l.split("MIP:")[1].strip()[:110])
        elif l.startswith("MIP: after root heuristics"):
            cur["heur_end"] = t
        elif l.startswith("MIP: background cuts: waited"):
            cur["bgwait"] += float(re.search(r"waited (\S+)s", l).group(1)); cur["bgend"] = t
        elif re.match(r"MIP: restart \d+ \(", l):
            cur["end"] = t; cur["end_kind"] = "restart"; phases.append(cur); cur = new()
        elif l.startswith("MIP: tree search with"):
            cur["tree_start"] = t
        elif l.startswith("MIP: finish done") or l.startswith("MIP: finish start"):
            cur["end"] = t; cur["end_kind"] = "finish"
        elif l.startswith("MIP: solve returned"):
            total = float(re.search(r"after (\S+)s", l).group(1))
    phases.append(cur)
    return phases, total
if __name__ == "__main__":
    for p in sys.argv[1:]:
        ph, tot = parse(p)
        off = 0.0
        print(f"== {p}  total {tot}")
        for k, c in enumerate(ph):
            f = lambda v: "-" if v is None else f"{v:.2f}"
            print(f" 段{k}: 開始 {off:.2f} | 根LP {f(c['rootlp'])} lb {c['lb0']} | カット {c['rounds']}回 終わり {f(c['cutend'])} lb {c['lb1']} | ヒューリ終わり {f(c['heur_end'])} | 裏のカット待ち {c['bgwait']:.2f} (至 {f(c['bgend'])}) | 木 {f(c.get('tree_start'))} | 終わり {f(c['end'])} {c['end_kind']}")
            for i in c["inc"]: print(f"    解 {off + i[0]:.2f}s (段内 {i[0]:.2f}) {i[1]:.6g} [{i[2]}] node {i[3]}")
            hs = [h for h in c["heur"] if h[2] >= 0.02 or h[1] != "none"]
            if hs: print("    ヒューリ(>=0.02s か解あり):", ", ".join(f"{h[0]} {h[1]} {h[2]:.2f}" for h in hs))
            off += c["end"] or 0.0
