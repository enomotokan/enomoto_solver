# b_<名前>_*.log から: 木に入った実時間 (求解の開始から最初の経過の行まで)、終わった時刻、ノード数、下界
import sys, glob, re, ast
for pat in sys.argv[1:]:
    for f in sorted(glob.glob(f"/home/user/enomoto_solver/work4/b_{pat}_*.log")):
        tree = None; res = None; t0 = None
        for line in open(f):
            if t0 is None and "Presolve:" in line: t0 = float(line[1:8]) - float(line.rsplit("(",1)[1].split("s)")[0])
            m = re.match(r"\[ *([\d.]+)\] +[\d.]+s +\d+ +\d+ ", line)
            if m and tree is None: tree = round(float(m.group(1)) - t0, 2)
            if "RESULT" in line:
                d = ast.literal_eval(line.split("RESULT ",1)[1].split(" ",1)[1]); t = float(line.split("RESULT ",1)[1].split(" ",1)[0])
                res = (t, d["status"], d["nodes"], d["best_bound"])
        print(f.split("/")[-1], "tree_start", tree, "res", res)
