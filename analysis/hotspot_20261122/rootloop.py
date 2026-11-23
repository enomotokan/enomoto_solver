"""根のカットのループの内訳: perf script の呼び出しの鎖から、`root_cut_loop_inner` の下にあるサンプルを、ループから見た直下の処理と
分離の種類に振り分ける。python rootloop.py /tmp/.../perf_<問題>_root.data → 割合の表 (ループ全体に対する %)。"""
import re, subprocess, sys
from collections import Counter
from dem import name
P = "/usr/lib/linux-tools-6.8.0-151/perf"
# 葉から根へ向かう鎖で最初に当たった段に入れる (上から優先)。分離の種類は関数名から
CATS = [
    ("LP の解き直し", r"slope_intercept_dual|TwoStageLp::solve|solve_once|solve_inner|lu::|FtLu|LuFactors|basis_kernel|PriceMat"),
    ("分離: 経路の集約 (cmir)", r"path_aggregation"),
    ("分離: 行 1 本ずつ (cmir・被覆・flow cover)", r"rows_cmir|separate_rows|row_sep|gen_cuts_parallel"),
    ("分離: tableau (gomory)", r"gomory|tableau|gmi"),
    ("分離: zerohalf", r"zerohalf|zero_half"),
    ("分離: クリーク・含意", r"clique|implied|implbd"),
    ("分離: hnf・mixing など", r"hnfcut|mixing|knapsack|lifted|vb_cover|flow"),
    ("分離: カットの仕上げ", r"finish_cut"),
    ("分離: その他 (cuts::)", r"cuts::|separate"),
    ("カットの選び (平行度・点数)", r"select_cuts|parallel|efficacy|cutsel|score"),
    ("プールの管理", r"pool|age|remove_cuts|add_cut_rows|add_rows"),
    ("伝播・固定", r"propagate|Domain::|redcost|fixing"),
    ("メモリ確保・複写", r"malloc|_mi_|mi_free|memmove|memcpy|memset"),
]


def main(data):
    out = subprocess.run([P, "script", "-i", data, "-F", "ip,sym", "--no-demangle"], capture_output=True, text=True).stdout
    cnt, tot, loop = Counter(), 0, 0
    for block in out.split("\n\n"):
        frames = [l.split(None, 1)[1].strip() if len(l.split(None, 1)) > 1 else "" for l in block.strip().splitlines()]
        if not frames: continue
        tot += 1
        names = [name(f) for f in frames]
        try:
            i = next(k for k, n in enumerate(names) if "root_cut_loop_inner" in n)
        except StopIteration:
            continue
        loop += 1
        below = names[:i]  # 葉からループの直前まで
        for c, pat in CATS:
            if any(re.search(pat, n) for n in below):
                cnt[c] += 1; break
        else:
            cnt["その他 (ループの関数そのもの・ほか)"] += 1
    print(f"サンプル {tot}、うち根のカットのループ {loop} ({100 * loop / max(tot, 1):.1f}%)")
    for c, v in cnt.most_common():
        print(f"  {100 * v / max(loop, 1):5.1f}%  {c}")


if __name__ == "__main__":
    main(sys.argv[1])
