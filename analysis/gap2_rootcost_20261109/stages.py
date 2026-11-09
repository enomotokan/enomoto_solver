"""ENOMOTO_MIP_LOG と HiGHS のログから段ごとの時間を出す (GAP2、2026-11-09)。
使い方: python stages.py ログ... (ours のログは名前に _ours_、HiGHS は _highs_)。
ENOMOTO の MIP の時計は前処理の後から数え、再スタートのたびに 0 に戻るので、区間 (根 1 回分) ごとに積む。
区間の中の段:
  pre   : 区間の始め〜「solving the root LP」(前処理の後のプロービング・LP プロービング・対称性など。1 区間目は MIP の時計の前の前処理を別に出す)
  rootlp: 「solving the root LP」〜「root LP <値> (... iters, t)」
  cuts  : 根の LP の後〜「cut loop done」
  heur  : 「cut loop done」〜「after root heuristics」(FJ の始まり・find thread・根の発見法 (RENS・RINS・サブ MIP の待ちを含む))
  bgwait: 「after root heuristics」〜「background cuts: ... waited」の行
  rest  : その後〜「restart N」または「tree search with」(根の目的の probing・再スタートの判定など)
  tree  : 「tree search with」〜「finish done」
"""
import re, sys
from pathlib import Path

T = re.compile(r"\(([0-9.]+)s[,)]")

def tlast(line):
    m = T.findall(line)
    return float(m[-1]) if m else None

def ours(path):
    lines = Path(path).read_text(errors="replace").splitlines()
    segs, cur = [], None
    total = None
    def new():
        return {"t0": 0.0, "lp": None, "lp_s": None, "lp_e": None, "cut_e": None, "heur_e": None, "bg_e": None, "end": None,
                "tree_s": None, "rounds": 0, "kind": None, "bgwait": 0.0, "lpiters": None, "last": 0.0}
    cur = new()
    for ln in lines:
        t = tlast(ln) if ln.startswith("MIP:") else None
        if t is not None and t < 1e5:
            cur["last"] = max(cur["last"], t) if not ln.startswith("MIP: solve returned") else cur["last"]
        if "MIP: solving the root LP" in ln and cur["lp_s"] is None:
            cur["lp_s"] = t
        elif re.match(r"MIP: root LP [-0-9]", ln) and cur["lp_e"] is None:
            cur["lp_e"] = t
            m = re.search(r"\((\d+) iters", ln); cur["lpiters"] = int(m.group(1)) if m else None
        elif re.match(r"MIP: cut round \d+: \d+ cuts", ln):
            cur["rounds"] += 1
        elif "MIP: cut loop done" in ln and cur["cut_e"] is None:
            cur["cut_e"] = cur["last"]
        elif "MIP: after root heuristics" in ln:
            cur["heur_e"] = t
        elif re.match(r"MIP: background cuts: determ: waited", ln) or re.match(r"MIP: background cuts: \d+ rounds", ln):
            if cur["bg_e"] is None:
                cur["bg_e"] = t
                m = re.search(r"waited ([0-9.]+)s", ln); cur["bgwait"] = float(m.group(1)) if m else 0.0
        elif re.match(r"MIP: restart \d+ \(", ln):
            cur["end"] = t; cur["kind"] = "restart"; segs.append(cur); cur = new()
        elif "MIP: tree search with" in ln:
            cur["tree_s"] = t
        elif "MIP: finish done" in ln:
            cur["end"] = t; cur["kind"] = "final"
        m = re.match(r"MIP: solve returned \S+ after ([0-9.]+)s", ln)
        if m:
            total = float(m.group(1))
    if cur["end"] is None:
        cur["end"] = cur["last"]; cur["kind"] = cur["kind"] or "final"
    segs.append(cur)
    out = []
    for s in segs:
        e = s["end"]
        lp_s = s["lp_s"] if s["lp_s"] is not None else e
        lp_e = s["lp_e"] if s["lp_e"] is not None else lp_s
        cut_e = s["cut_e"] if s["cut_e"] is not None else lp_e
        heur_e = s["heur_e"] if s["heur_e"] is not None else cut_e
        bg_e = s["bg_e"] if s["bg_e"] is not None else heur_e
        root_e = s["tree_s"] if s["tree_s"] is not None else e
        out.append({"pre": lp_s, "rootlp": lp_e - lp_s, "cuts": max(0, cut_e - lp_e), "rounds": s["rounds"],
                    "heur": max(0, heur_e - cut_e), "bgwait": max(0, bg_e - heur_e), "rest": max(0, root_e - max(bg_e, cut_e)),
                    "tree": (e - s["tree_s"]) if s["tree_s"] is not None else 0.0, "len": e, "kind": s["kind"], "lpiters": s["lpiters"]})
    js = [l for l in lines if l.startswith("{")]
    import json
    res = json.loads(js[-1]) if js else {}
    seglen = sum(o["len"] for o in out)
    return {"segs": out, "total": res.get("time", total), "premip": (total or 0) - seglen, "res": res}

def highs(path):
    txt = Path(path).read_text(errors="replace")
    import json
    d = {}
    m = re.search(r"Timing\s+([0-9.]+)\s+([0-9.]+) \(Presolve\)\s+MIP\s+time \[calls\] = ([0-9.]+) \[\d+\]\s+subMIP time \[calls\] = ([0-9.]+) \[(\d+)\]\s+([0-9.]+) \(Solve\)\s+MIP\s+time \[calls\] = ([0-9.]+) \[\d+\]\s+subMIP time \[calls\] = ([0-9.]+) \[(\d+)\]", txt)
    if m:
        d.update(total=float(m.group(1)), presolve=float(m.group(2)), solve=float(m.group(6)), submip_solve=float(m.group(8)),
                 submip_pre=float(m.group(4)), submip_calls=int(m.group(9)))
    d["restarts"] = len(re.findall(r"restarting", txt))
    # 根の終わり: 木の表で Proc. が 0 でない最初の行の時刻 (ないなら最後)
    rows = re.findall(r"^ ([A-Za-z ]) +(\d+) +(\d+) +(\d+) +[0-9.]+% .* ([0-9.]+)s$", txt, re.M)
    first_lp = None
    rootend = None
    for src, proc, q, lv, tm in rows:
        if int(proc) > 0 and rootend is None:
            rootend = float(tm)
    lines = [l for l in txt.splitlines() if re.match(r"^ [A-Za-z ] +\d+ +\d+ +\d+ +[0-9.]+%", l)]
    d["rootend"] = rootend
    d["rows"] = len(lines)
    m = re.search(r"LP iterations\s+(\d+)\s+(\d+) \(strong br.\)\s+(\d+) \(separation\)\s+(\d+) \(heuristics\)", txt)
    if m:
        d.update(lpit=int(m.group(1)), sbit=int(m.group(2)), sepit=int(m.group(3)), heurit=int(m.group(4)))
    m = re.search(r"Nodes\s+(\d+)", txt); d["nodes"] = int(m.group(1)) if m else None
    return d

if __name__ == "__main__":
    for p in sys.argv[1:]:
        if "_highs_" in p:
            print(p, highs(p))
        else:
            r = ours(p)
            print(p, "total", r["total"], "premip %.2f" % r["premip"])
            for s in r["segs"]:
                print("   ", {k: (round(v, 2) if isinstance(v, float) else v) for k, v in s.items()})
