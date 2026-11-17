"""HiGHS で「プロービング + 規則 X」を止めて前処理した問題を書き出し、当方で解いて前処理の後の大きさ・根の LP・結果を見る。
H115 (highspy 1.15.1 の python) で書き出し、リポジトリの venv の python で当方を回す。
python rulecut.py 元の.mps 出力の置き場 [秒] (rulecut.txt に追記)"""
import os, re, subprocess, sys
mps, out = sys.argv[1], sys.argv[2]; tl = sys.argv[3] if len(sys.argv) > 3 else "40"
H115 = os.environ["H115"]; WT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
RULES = {"none": 0, "dominated": 5, "forcing_row": 6, "forcing_col": 7, "free_subst": 8, "doubleton_eq": 9, "dep_eq": 10,
         "dep_free": 11, "aggregator": 12, "parallel": 13, "sparsify": 14, "enumeration": 16, "dual_fixing": 17,
         "zero_cost_singleton": 18, "stuffing": 19, "dualfix_probing": 20, "fourier_motzkin": 22, "weak_dominated": 23}
W = """
import highspy, sys
h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('threads', 1)
h.setOptionValue('presolve_rule_off', int(sys.argv[3])); h.readModel(sys.argv[1]); h.presolve(); h.writePresolvedModel(sys.argv[2])
"""
for name, bit in RULES.items():
    mask = (1 << 15) | ((1 << bit) if name != "none" else 0)
    f = os.path.join(out, f"np_{name}.mps")
    subprocess.run([H115, "-I", "-c", W, mps, f, str(mask)], check=True)
    hd = subprocess.run([H115, "-I", "-c", "import highspy,sys;h=highspy.Highs();h.setOptionValue('output_flag',False);h.readModel(sys.argv[1]);l=h.getLp();print(l.num_row_,l.num_col_)", f], capture_output=True, text=True).stdout.split()
    env = dict(os.environ, ENOMOTO_MIP_LOG="1")
    r = subprocess.run([os.path.join(WT, ".venv/bin/python"), os.path.join(WT, "scripts/enomoto_mip_cli.py"), f, "--time-limit", tl, "--threads", "4", "--log"],
                       capture_output=True, text=True, env=env)
    pre = re.findall(r"Presolve: .* -> (\d+) rows, (\d+) columns", r.stdout)
    root = re.findall(r"Root LP: objective (\S+)", r.stdout)
    st = re.findall(r"^status\s+(\S+)", r.stdout, re.M); bb = re.findall(r"^best_bound\s+(\S+)", r.stdout, re.M)
    eq = re.findall(r"probing: (\d+) equivalent binary pairs", r.stderr)
    tm = re.findall(r"^time_solve\s+(\S+)", r.stdout, re.M)
    line = f"{name:22s} HiGHS {hd[0]:>6} 行 {hd[1]:>6} 列 | 当方 前処理後 {pre[0] if pre else '-'} 根 {root[0] if root else '-'} 同値 {eq[0] if eq else '-'} | {st[0] if st else '-'} 下界 {bb[0] if bb else '-'} {tm[0] if tm else '-'}s"
    print(line, flush=True)
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "rulecut.txt"), "a") as fo:
        fo.write(line + "\n")
