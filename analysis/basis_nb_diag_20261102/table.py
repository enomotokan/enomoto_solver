"""logs/*.log (NB_DIAG の行) から表を作る (Markdown を標準出力に)。

使い方: python analysis/basis_nb_diag_20261102/table.py > analysis/basis_nb_diag_20261102/tables.md
"""
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "analysis" / "cbcimpl_20261029"))
import probs  # noqa: E402

SITES = ["tree_sibling", "tree_queue", "tree_pool", "submip_root", "bg_copy", "bg_return", "restart", "root_probe", "tree_root", "slack_retry", "untagged", "no_set_basis"]
TREE = ["tree_sibling", "tree_queue", "tree_pool"]


def load(p):
    d = {"total": None, "res": {}, "s": {}}
    f = HERE / "logs" / f"{p}.log"
    if not f.exists():
        return None
    for line in f.read_text().splitlines():
        if line.startswith("NB_DIAG total"):
            d["total"] = float(line.split()[2])
        elif line.startswith("NB_DIAG "):
            w = line.split()
            kv = {k: float(v) for k, v in (x.split("=") for x in w[3:])}
            d["s"][(w[1], w[2])] = kv
        elif line.startswith("RESULT "):
            d["res"] = json.loads(line[7:])
    return d


def agg(rows):
    """複数の行を足す"""
    out = defaultdict(float)
    for r in rows:
        for k, v in r.items():
            out[k] = max(out[k], v) if k == "max_it" else out[k] + v
    return out


def fmt_site(s):
    n, nm, nc = s["n"], s["n_mis"], s["n_cmp"]
    n_ok = n - nm
    it_mis = s["it_mis"] / nm if nm else float("nan")
    it_ok = s["it_ok"] / n_ok if n_ok else float("nan")
    return (f"{int(n)} | {100 * nm / n:.0f}% | {s['sum_cmp'] / nc if nc else 0:.0f} | {(s['sum_opp'] + s['sum_unk']) / nc if nc else 0:.1f} "
            f"({s['sum_opp'] / nc if nc else 0:.1f}/{s['sum_unk'] / nc if nc else 0:.1f}) | {s['sum_eff'] / nc if nc else 0:.1f} | {s['sum_fix'] / nc if nc else 0:.2f} | "
            f"{100 * s['n_fix'] / n:.0f}% | {it_mis:.0f} | {it_ok:.0f} | {s['it_fix'] / s['n_fix'] if s['n_fix'] else float('nan'):.0f} | "
            f"{s['t_mis'] + s['t_ok']:.2f} | {s['t_fix']:.2f} | {int(s['max_it'])}")


HDR = ("| LP 数 | 食い違いのある LP | 比べた列・行 (平均) | 食い違い 平均 (反対/不明) | 実際に置いた側の食い違い 平均 | うち保存した側に置けた 平均 | "
       "置けた列のある LP | 反復 平均 (食い違いあり) | 反復 平均 (なし) | 反復 平均 (置けた列あり) | 時間 計 (秒) | 時間 計 (置けた列あり) | 最大反復 |")
SEP = "|" + "---|" * 15


def main():
    data = {p: load(p) for p in probs.PROBS}
    data = {p: d for p, d in data.items() if d}
    print(f"問題数: {len(data)}\n")
    # 1. 場所ごとの 58 問の合計 (main / sub)
    print("## 場所ごとの合計 (全問題)\n")
    print("| 場所 " + HDR)
    print("|---" + SEP)
    for lvl in ["main", "sub"]:
        for site in SITES:
            rows = [d["s"][(lvl, site)] for d in data.values() if (lvl, site) in d["s"]]
            if rows:
                print(f"| {lvl} {site} | " + fmt_site(agg(rows)) + " |")
    # 2. 問題ごとの要約 (main の木 = tree_*、sub の全部、写し・サブ MIP の根)
    print("\n## 問題ごとの要約\n")
    print("木 = main の tree_sibling + tree_queue + tree_pool。LP 時間 = main の全 LP の時間の和 (スレッドの和なので経過時間を超えうる)。"
          "木の割合 = 木の set_basis の後の LP の時間 / main の全 LP の時間。\n")
    print("| 問題 | 結果 | 経過 (秒) | main LP 時間 | 木 LP 数 | 木 食い違い LP | 木 食い違い 平均 | 木 反復 (あり/なし) | 木 時間 | 木の割合 | 木 置けた列あり LP | 木 時間 (置けた列あり) | "
          "サブ根 LP 数 | サブ根 時間 | サブ根 最大反復 | 写し LP 数 | 写し 時間 | 写し 最大反復 | 再スタート 反復 |")
    print("|" + "---|" * 19)
    summ = []
    for p, d in data.items():
        s = d["s"]
        main_all = agg([v for (l, _), v in s.items() if l == "main"])
        tree = agg([s[("main", t)] for t in TREE if ("main", t) in s])
        sr = agg([v for (l, site), v in s.items() if site == "submip_root"])
        bg = s.get(("main", "bg_copy"), {})
        rs = s.get(("main", "restart"), {})
        t_lp = main_all.get("t_mis", 0) + main_all.get("t_ok", 0)
        t_tree = tree.get("t_mis", 0) + tree.get("t_ok", 0)
        n_t = tree.get("n", 0)
        nm = tree.get("n_mis", 0)
        nc = tree.get("n_cmp", 0)
        itm = tree["it_mis"] / nm if nm else float("nan")
        ito = tree["it_ok"] / (n_t - nm) if n_t - nm else float("nan")
        r = d["res"]
        summ.append((p, t_tree, t_lp, tree.get("t_fix", 0)))
        print(
            f"| {p} | {r.get('status')} | {r.get('time') or 0:.1f} | {t_lp:.1f} | {int(n_t)} | {100 * nm / n_t if n_t else 0:.0f}% | "
            f"{(tree.get('sum_opp', 0) + tree.get('sum_unk', 0)) / nc if nc else 0:.1f} | {itm:.0f}/{ito:.0f} | {t_tree:.2f} | {100 * t_tree / t_lp if t_lp else 0:.0f}% | "
            f"{int(tree.get('n_fix', 0))} | {tree.get('t_fix', 0):.2f} | "
            f"{int(sr.get('n', 0))} | {sr.get('t_mis', 0) + sr.get('t_ok', 0):.2f} | {int(sr.get('max_it', 0))} | "
            f"{int(bg.get('n', 0))} | {bg.get('t_mis', 0) + bg.get('t_ok', 0):.2f} | {int(bg.get('max_it', 0))} | {int(rs.get('max_it', 0))} |"
        )
    # 3. 問題ごと・場所ごとの詳細
    print("\n## 問題ごと・場所ごとの詳細\n")
    print("| 問題 | 場所 " + HDR)
    print("|---|---" + SEP)
    for p, d in data.items():
        for lvl in ["main", "sub"]:
            for site in SITES:
                if site == "no_set_basis":
                    continue
                if (lvl, site) in d["s"]:
                    print(f"| {p} | {lvl} {site} | " + fmt_site(d["s"][(lvl, site)]) + " |")


main()
