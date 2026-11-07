"""根のカットのループのログ (ENOMOTO_MIP_LOG=1 の stderr) から、主の最初の根のカットのループのラウンドごとの様子を拾う。
parse(text) -> dict: rounds (ラウンドごと: 本数・候補・LP 反復・上限・上限に当たったか・後の扱い・目的値)、end (ループの終わり方)、obj0・obj1、sense"""
import re

R_OK = re.compile(r"MIP: cut round (\d+): (\d+) cuts \(of (\d+), sep [\d.]+s, LP (\d+) iters\), LP rows (\d+).*obj (\S+) \(([\d.]+)s\)")
R_PROG = re.compile(r"MIP: cut round (\d+): cut LP stopped by the progress check at iter (\d+) \(cap (\d+)\): window gain (\S+), gain from the start (\S+) \((\d+) iters\)")
R_PAST = re.compile(r"MIP: cut round (\d+): cut LP went past the cap (\d+) on progress: (\w+) after (\d+) iters")
R_FAIL = re.compile(r"MIP: cut round (\d+): LP status (\w+), removed the cuts \((\w+); cut LP (\d+) iters ([\d.]+)s, re-solve (\d+) iters ([\d.]+)s, ([\d.]+)s\)")
R_CAPLOG = re.compile(r"MIP: cut round (\d+): CUTLPCAP cap (\d+) cuts (\d+) rows (\d+)")
R_DONE = re.compile(r"MIP: cut loop done: (\d+) cuts added, (\d+) in LP.*obj (\S+) -> (\S+)")
R_ROOTCUTS = re.compile(r"Root cuts: objective (\S+) -> (\S+),")
R_ROOTLP = re.compile(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)")


def parse(text):
    lines = text.splitlines()
    out = {"rounds": {}, "end": None, "obj0": None, "obj1": None, "sense": 1, "root_lp_iters": None, "fails": []}
    rd = out["rounds"]
    started = False
    last_round = None
    for i, l in enumerate(lines):
        if out["root_lp_iters"] is None:
            m = R_ROOTLP.search(l)
            if m:
                out["root_lp_iters"] = int(m.group(2)); out["root_lp"] = float(m.group(1))
        if "background" in l or "SUBMIP" in l or "sub-MIP" in l:
            continue
        m = R_CAPLOG.search(l)
        if m:
            started = True
            r = int(m.group(1)); rd.setdefault(r, {})["cap"] = int(m.group(2)); rd[r]["ncuts"] = int(m.group(3))
            continue
        m = R_OK.search(l)
        if m:
            started = True
            r = int(m.group(1)); last_round = r
            d = rd.setdefault(r, {})
            d.update(ncuts=int(m.group(2)), cands=int(m.group(3)), iters=int(m.group(4)), rows=int(m.group(5)), obj=float(m.group(6)), t=float(m.group(7)), hit=False)
            continue
        m = R_PROG.search(l)
        if m:
            started = True
            r = int(m.group(1)); d = rd.setdefault(r, {})
            d.update(prog_it=int(m.group(2)), cap=int(m.group(3)), win_gain=float(m.group(4)), tot_gain=float(m.group(5)))
            continue
        m = R_PAST.search(l)
        if m:
            r = int(m.group(1)); d = rd.setdefault(r, {}); d.update(cap=int(m.group(2)), past=True)
            continue
        m = R_FAIL.search(l)
        if m:
            started = True
            r = int(m.group(1)); last_round = r; d = rd.setdefault(r, {})
            d.update(hit=True, status=m.group(2), resolve=m.group(3), iters=int(m.group(4)), lp_secs=float(m.group(5)), t=float(m.group(8)), action="drop")
            out["fails"].append(r)
            # 後の扱い: 次の数行に止めの印があるか
            for l2 in lines[i + 1:i + 6]:
                if "stop (ENOMOTO_MIP_CUT_LP_CAP_STOP)" in l2:
                    d["action"] = "drop+stop(CAP_STOP)"
                    out["end"] = "cap_stop"
                if "FLAT_STOP" in l2:
                    d["action"] = "drop+stop(FLAT)"
                    out["end"] = "flat_stop"
            continue
        if started and out["end"] is None:
            if "-> stop" in l and "root cut time cap check" in l:
                out["end"] = "time_cap"
            elif "stall" in l.lower() and "stop" in l and "cut" in l:
                out["end"] = "stall"
            elif "progress of the last" in l and "stop" in l:
                out["end"] = "progress"
        m = R_ROOTCUTS.search(l)
        if m and started and out["obj1"] is None:
            a, b = float(m.group(1)), float(m.group(2))
            out["orig0"], out["orig1"] = a, b
        if started and ("MIP: root heuristic" in l or "MIP: after root heuristics" in l):
            if out["fails"] and out["end"] is None and rd[out["fails"][-1]].get("action") == "drop":
                out["end"] = "fail_return"
                rd[out["fails"][-1]]["action"] = "drop+return(2 fails)"
            break
        m = R_DONE.search(l)
        if m and started:
            out["obj0"], out["obj1"] = float(m.group(3)), float(m.group(4))
            if "orig0" in out and abs(out["orig0"] + out["obj0"]) < abs(out["orig0"] - out["obj0"]):
                out["sense"] = -1
            if out["end"] is None:
                out["end"] = "other"
            break
    oks = [d for r, d in sorted(rd.items()) if "obj" in d]
    if out["obj1"] is None:
        out["obj1"] = max([d["obj"] for d in oks], default=out.get("root_lp"))
        out["obj0"] = out.get("root_lp")
    if out["end"] is None:
        out["end"] = "other" if started else "no_loop"
    return out
