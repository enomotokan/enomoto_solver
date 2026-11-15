"""TREECLOSE の読み取り: ENOMOTO (ENOMOTO_MIP_LOG=1) と HiGHS のログから、時間に沿った下界・上界・ノード数と、終わりの統計を抜き出す。
python parse.py LOG [LOG ...]  (ファイル名に _highs を含むものは HiGHS のログとして読む)。各ログに 1 行の JSON を出す。"""
import gzip, json, re, sys

NUM = r"[-+]?(?:\d+\.?\d*(?:[eE][-+]?\d+)?|inf)"
CK = [10, 30, 60, 120, 240, 400, 600]


def opn(f):
    return gzip.open(f, "rt", errors="replace") if f.endswith(".gz") else open(f, errors="replace")


def eno(f):
    off, seg_nodes, tot_nodes = 0.0, 0, 0
    ub, lb = float("inf"), -float("inf")
    pts, inc, st = [], [], {}
    restarts, tree_start = 0, None
    tc = []
    for line in opn(f):
        m = re.match(r"TCTRACE outer (\S+) solve \S+ restarts \d+ nodes (\d+) open (\d+) glb (\S+) ub (\S+)", line)
        if m:
            tc.append([float(m.group(1)), float(m.group(4)), float(m.group(5)), int(m.group(2)), int(m.group(3))])
            continue
        m = re.match(r"TRSTAT: tree start outer (\S+) solve (\S+) restarts (\d+)", line)
        if m:
            off = float(m.group(1)) - float(m.group(2))
            tot_nodes += seg_nodes
            seg_nodes = 0
            restarts = int(m.group(3))
            if tree_start is None:
                tree_start = float(m.group(1))
            continue
        m = re.match(r"MIP:\s+(\S+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+) lp_iters (\d+)", line)
        if m:
            t = float(m.group(1)) + off
            seg_nodes = int(m.group(2))
            l = float(m.group(4))
            u = float(m.group(5))
            if l != float("inf"):
                lb = max(lb, l)
            ub = min(ub, u)
            pts.append((t, lb, ub, tot_nodes + seg_nodes, int(m.group(3))))
            continue
        m = re.match(r"MIP: new incumbent (\S+) at node \d+ \((\S+)s\)(.*)", line)
        if m:
            z = float(m.group(1))
            t = float(m.group(2)) + off
            if z < ub:
                ub = z
                inc.append((round(t, 1), z, m.group(3).strip()))
            continue
        for key, pat in [("node_diag", r"MIP: node diag: (.*)"), ("nbdiag", r"MIP: basis nb diag: (.*)"), ("branching", r"MIP: branching: (.*)"),
                         ("threads", r"MIP: tree search threads: (.*)"), ("restores", r"MIP: sibling backtracks (.*)"), ("adaptcuts", r"MIP: adaptive node cuts: (.*)"),
                         ("proofs", r"MIP: dual proofs and conflicts (.*)"), ("result", r"MIP: solve returned (.*)"), ("orbital", r"MIP: orbital fixing (.*)"),
                         ("orbitopes", r"MIP: orbitopes (.*)")]:
            m = re.match(pat, line)
            if m:
                st[key] = m.group(1).strip()[:400]
        m = re.match(r"MIP: (?:TREE )?restart (\d+) \((.*)", line)
        if m:
            st.setdefault("restart_lines", []).append(line.strip()[:200])
        if "| tid " in line:
            st.setdefault("prof", []).append(line.strip()[line.find("| tid"):][:400])
    if tc:
        # TCTRACE があればそれを使う (全体の下界・全スレッドのノード数)。再スタートの後のノード数は前の分を足し、上界・下界は単調にする
        pts, base, prev, lo, hi = [], 0, 0, -float("inf"), float("inf")
        for t, l, u, n, o in tc:
            if n < prev:
                base += prev
            prev = n
            hi = min(hi, u)
            if l != float("inf") and l == l:
                lo = max(lo, min(l, hi))
            pts.append((t, lo, hi, base + n, o))
    return dict(pts=pts, inc=inc, restarts=restarts, tree_start=tree_start, stats=st, tctrace=bool(tc))


HL = re.compile(r"^\s?([A-Za-z ])\s+(\d+k?)\s+(\d+k?)\s+(\d+k?)\s+(\S+)%\s+(\S+)\s+(\S+)\s+(\S+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+k?)\s+(\S+)s")


def kint(s):
    return int(float(s[:-1]) * 1000) if s.endswith("k") else int(s)


def highs(f):
    pts, inc, st = [], [], {}
    ub, lb, restarts, base_nodes, last_nodes = float("inf"), -float("inf"), 0, 0, 0
    lines = list(opn(f))
    for i, line in enumerate(lines):
        if line.startswith("Restarting search") or line.startswith("Model after restart"):
            if line.startswith("Model after restart"):
                restarts += 1
                st.setdefault("restart_lines", []).append(line.strip()[:200])
            continue
        m = HL.match(line)
        if m:
            src = m.group(1).strip()
            nodes = kint(m.group(2))
            try:
                l = float(m.group(6))
                u = float(m.group(7)) if m.group(7) != "inf" else float("inf")
            except ValueError:
                continue
            t = float(m.group(13))
            lb = max(lb, l)
            if u < ub:
                ub = u
                inc.append((t, u, src))
            pts.append((t, lb, ub, nodes, kint(m.group(3))))
        for key in ["Status", "Nodes", "LP iterations", "Primal bound", "Dual bound"]:
            if line.strip().startswith(key + " "):
                st[key] = line.strip()[len(key):].strip()
                if key == "LP iterations":
                    st["lp_split"] = " / ".join(lines[j].strip() for j in range(i + 1, i + 4))
        m = re.match(r"\s+subMIP time \[calls\] = (\S+) \[(\d+)\]", line)
        if m:
            st.setdefault("submip", []).append(m.group(1) + "s/" + m.group(2))
        m = re.match(r"\s+(\S+) \(Solve\)", line)
        if m:
            st["solve_s"] = m.group(1)
    return dict(pts=pts, inc=inc, restarts=restarts, stats=st)


def at(pts, t):
    r = None
    for p in pts:
        if p[0] <= t:
            r = p
        else:
            break
    return r


if __name__ == "__main__":
    for f in sys.argv[1:]:
        d = highs(f) if "_highs" in f else eno(f)
        d["file"] = f
        d["ck"] = {t: at(d["pts"], t) for t in CK}
        d["last"] = d["pts"][-1] if d["pts"] else None
        del d["pts"]
        print(json.dumps(d))
