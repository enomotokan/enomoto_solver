"""ENOMOTO_MIP_DEBUG_RESTART_CUT_USE のログから、再スタートごとの経路・持ち込んだ数・根の LP の値を拾う。

使い方: python extract.py <ログ ...>  (1 行 1 回の再スタート、タブ区切り)"""
import re, sys
from pathlib import Path

for f in sys.argv[1:]:
    name = Path(f).stem
    last_obj = None
    rs = {}
    cur = None
    for line in open(f, errors="replace"):
        m = re.match(r"MIP: cut round \d+: .* obj (\S+) \(", line)
        if m:
            last_obj = float(m.group(1))
        m = re.match(r"MIP: root LP (\S+) \(", line)
        if m and last_obj is None:
            last_obj = float(m.group(1))
        m = re.match(r"RESTART_CUT_USE: restart (\d+) path (.*?): carried (\d+)", line)
        if m:
            n = int(m.group(1))
            cur = rs.setdefault(n, {})
            cur.update(path=m.group(2), carried=int(m.group(3)), before=last_obj)
            last_obj = None
        m = re.match(r"MIP: restart (\d+) \((\d+) of (\d+) integer columns newly fixed", line)
        if m:
            rs.setdefault(int(m.group(1)), {}).update(newly=int(m.group(2)))
        m = re.match(r"MIP: (\d+) cuts from before the restart (added back to the LP|put in the cut pool)", line)
        if m and cur is not None:
            cur["landed"] = ("readd " if "added" in m.group(2) else "pool ") + m.group(1)
        m = re.match(r"MIP: restart cuts mapped through the presolve by substitution: (\d+) of (\d+) kept \(without substitution (\d+)\)", line)
        if m and cur is not None:
            cur["subst"] = f"{m.group(1)}/{m.group(2)} (old {m.group(3)})"
        m = re.match(r"RESTART_CUT_USE: substitution check .* max slack difference (\S+), mapped cuts violated (\d+)", line)
        if m and cur is not None:
            cur["check"] = f"{m.group(1)} viol {m.group(2)}"
        m = re.match(r"RESTART_CUT_USE: after restart (\d+) root LP (\S+): cut pool (\d+) \(violated (\d+), tight (\d+), slack (\d+)\); LP cut rows (\d+)", line)
        if m:
            r = rs.setdefault(int(m.group(1)), {})
            if "after" not in r:
                r.update(after=float(m.group(2)), use=f"pool {m.group(3)} (v{m.group(4)}/t{m.group(5)}/s{m.group(6)}), cut rows {m.group(7)}")
            else:
                r["after_end"] = float(m.group(2))
        m = re.match(r"MIP: presolve: rows (\d+) -> (\d+), .* postsolve steps (\d+)", line)
        if m and cur is not None and "pre" not in cur:
            cur["pre"] = f"{m.group(1)}->{m.group(2)} rows, {m.group(3)} steps"
    if not rs:
        print(f"{name}\t-\tno restart")
    for n in sorted(rs):
        r = rs[n]
        fmt = lambda v: "-" if v is None else f"{v:.6g}"
        print("\t".join([name, str(n), r.get("path", "?"), str(r.get("carried", "")), str(r.get("newly", "")), r.get("landed", ""),
                         r.get("subst", ""), r.get("check", ""), fmt(r.get("before")), fmt(r.get("after")), fmt(r.get("after_end")), r.get("pre", ""), r.get("use", "")]))
