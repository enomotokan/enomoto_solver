"""fuzz_objint の種 491 を繰り返し解き、not_solved の回のログを書き出す: python oi491.py 回数 出力の接頭辞 (既定は種 491、ENOMOTO_OI_SEED で変える)"""
import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "objint_20261112"))
import fuzz_objint as fo  # noqa: E402
n, pre = int(sys.argv[1]), sys.argv[2]
seed = int(os.environ.get("ENOMOTO_OI_SEED", "491"))
cnt = {}
for i in range(n):
    st, obj, _ = fo.ours(fo.gen(seed))
    cnt[st] = cnt.get(st, 0) + 1
    if st != "optimal":
        open(f"{pre}_{i}.log", "w").write(fo.LASTLOG[0])
print(cnt)
