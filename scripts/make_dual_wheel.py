"""(c) の配り方の試作: base 版の wheel に v3 版の _core を _core_v3*.so として足し、__init__.py を var/dual のもの (CPU を見て読む方を選ぶ) に替えた wheel を作る。
RECORD (sha256・大きさ) も作り直すので pip でそのまま入る。
使い方: python make_dual_wheel.py BASE_WHEEL V3_WHEEL INIT_PY OUT_DIR"""
import base64, hashlib, os, sys, zipfile
bw, vw, init, out = sys.argv[1:5]
def h(b): return "sha256=" + base64.urlsafe_b64encode(hashlib.sha256(b).digest()).rstrip(b"=").decode()
with zipfile.ZipFile(vw) as z:
    so = next(n for n in z.namelist() if n.startswith("enomoto_solver/_core") and n.endswith(".so"))
    v3 = z.read(so)
v3name = so.replace("/_core.", "/_core_v3.")
os.makedirs(out, exist_ok=True)
dst = os.path.join(out, os.path.basename(bw))
files = []
with zipfile.ZipFile(bw) as z:
    rec = next(n for n in z.namelist() if n.endswith(".dist-info/RECORD"))
    for n in z.namelist():
        if n == rec: continue
        b = open(init, "rb").read() if n == "enomoto_solver/__init__.py" else z.read(n)
        files.append((n, b))
files.append((v3name, v3))
with zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as z:
    for n, b in files: z.writestr(n, b)
    z.writestr(rec, "".join(f"{n},{h(b)},{len(b)}\n" for n, b in files) + f"{rec},,\n")
print(dst, os.path.getsize(dst))
