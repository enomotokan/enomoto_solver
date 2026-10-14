import re, sys, glob, os
d = sys.argv[1]
def cpu_at(samples, t):
    # 実時間 t のときの CPU 時間 (線形補間)
    prev = samples[0]
    for s in samples:
        if s[0] >= t:
            if s[0] == prev[0]: return s[1]
            return prev[1] + (s[1] - prev[1]) * (t - prev[0]) / (s[0] - prev[0])
        prev = s
    return samples[-1][1]
print(f"{'問題':18s} {'前処理':>6s} {'根LP':>5s} {'カット':>6s} {'(分離)':>6s} {'ヒュー':>6s} {'主だけ':>6s} {'根合計':>6s} | コア数: {'前処理':>5s} {'カット':>5s} {'ヒュー':>5s} {'主だけ':>5s} {'木':>5s}")
for f in sorted(glob.glob(f"{d}/*.log")):
    name = os.path.basename(f)[:-4]
    L = open(f).read().splitlines()
    ss = float(next(l.split()[1] for l in L if l.startswith("SOLVE_START")))
    se = float(next(l.split()[1] for l in L if l.startswith("SOLVE_END")))
    tot = float(re.search(r"TREEPAR: total ([\d.]+)s", "\n".join(L)).group(1))
    m = re.search(r"tree start ([\d.]+)s workers start ([\d.]+)s", "\n".join(L))
    t_first, t_work = float(m.group(1)), float(m.group(2))
    pre = (se - ss) - tot  # ソルバーの時計が始まるまで (前処理・probing など)
    # 再スタートがあれば最後の区間だけを見る (時計が 0 に戻る)
    txt = "\n".join(L)
    rl = [float(x) for x in re.findall(r"root LP \S+ \(\d+ iters, ([\d.]+)s\)", txt)]
    cr = re.findall(r"cut round \d+: .*?sep ([\d.]+)s.*?\(([\d.]+)s\)$", txt, re.M)
    heur_end = [float(x) for x in re.findall(r"after root heuristics: incumbent .* \(([\d.]+)s\)", txt)]
    t_rootlp = rl[-1] if rl else 0.0
    t_cut_end = float(cr[-1][1]) if cr else t_rootlp
    sep = sum(float(a) for a, _ in cr)
    t_heur_end = heur_end[-1] if heur_end else t_cut_end
    samples = [tuple(map(float, l.split())) for l in open(f"{d}/{name}.cpu")]
    w0 = ss + pre  # ソルバーの時計 0 の実時間 (近似)
    def cores(a, b):
        wa, wb = w0 + a, w0 + b
        if wb - wa < 0.3: return float("nan")
        return (cpu_at(samples, wb) - cpu_at(samples, wa)) / (wb - wa)
    c_pre = (cpu_at(samples, w0) - cpu_at(samples, ss)) / max(pre, 1e-9)
    print(f"{name:18s} {pre:6.1f} {t_rootlp:5.1f} {t_cut_end - t_rootlp:6.1f} {sep:6.1f} {t_heur_end - t_cut_end:6.1f} {t_work - t_heur_end:6.1f} {pre + t_work:6.1f} | {c_pre:5.2f} {cores(t_rootlp, t_cut_end):5.2f} {cores(t_cut_end, t_heur_end):5.2f} {cores(t_heur_end, t_work):5.2f} {cores(t_work + 1, tot - 1):5.2f}")
