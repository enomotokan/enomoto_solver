"""hypothyroid のログから時刻を抜き出す: python hypo_timeline.py <ログ> ...
時刻はソルバーの時計 (前処理の後から) と、壁時計 (solve の呼び出しから。最初の解の時刻のずれで直す) の両方。"""
import json, re, sys

for f in sys.argv[1:]:
    L = open(f).read().splitlines()
    js = json.loads(L[-1]) if L and L[-1].startswith("{") else {}
    def t(pat):
        for l in L:
            if re.search(pat, l):
                m = re.findall(r"([0-9.]+)s\)", l)
                return float(m[-1]) if m else None
        return None
    fj = t(r"new incumbent .*(feasibility jump|FJ|Local-MIP|mix)")
    off = (js.get("first_solution_time") or 0) - fj if fj is not None and js.get("first_solution_time") else None
    pre = t(r"^Presolve:")
    rows = {
        "presolve": pre, "root LP": t(r"^MIP: root LP -"), "cuts done": t(r"^Root cuts:"),
        "-2851 found": t(r"new incumbent -2.851"), "early RENS": t(r"early RENS before"),
        "proved": t(r"proved the incumbent"), "total(wall)": js.get("time"), "offset": off,
    }
    src = next((re.search(r"\[(.*)\]", l).group(1) for l in L if "new incumbent -2.851" in l), None)
    print(f.split("/")[-1], " ".join(f"{k}={v:.2f}" if isinstance(v, float) else f"{k}={v}" for k, v in rows.items()), "src=", src, js.get("status"))
