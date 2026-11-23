"""(c) の試作: 基本版と v3 版の 2 つの .so を同じ包みに入れ、__init__.py で CPU を見て読む方を選ぶ (analysis/buildopt_20261122)。
python make_dual.py BASE V3 OUT → var/OUT/enomoto_solver/ (var/BASE の .py と _core*.so、var/V3 の .so を _core_v3*.so として、__init__.py に選ぶ処理を足す)。
- 選び方: Linux の /proc/cpuinfo の flags に x86-64-v3 の機能 (avx avx2 bmi1 bmi2 fma f16c movbe abm(lzcnt)) が全部あれば v3 を読む。
  読めない・他の OS・`ENOMOTO_CORE=base` なら基本版。`ENOMOTO_CORE=v3` で強制。
- v3 の .so も中の初期化関数は PyInit__core なので、importlib で名前 enomoto_solver._core・ファイル _core_v3*.so として読み、sys.modules に入れる。
  これで後の `from ._core import ...`・`from . import _core` はそのまま v3 を使う。"""
import shutil, sys
from pathlib import Path
D = Path(__file__).resolve().parent
B, V, O = sys.argv[1], sys.argv[2], sys.argv[3]
o = D / "var" / O / "enomoto_solver"
if o.exists(): shutil.rmtree(o)
shutil.copytree(D / "var" / B / "enomoto_solver", o)
v3so = next((D / "var" / V / "enomoto_solver").glob("_core*.so"))
shutil.copy(v3so, o / v3so.name.replace("_core", "_core_v3", 1))
LOADER = '''
def _enomoto_load_core():
    """CPU が x86-64-v3 に対応していれば同梱の _core_v3 を enomoto_solver._core として読む (analysis/buildopt_20261122 の (c) の試作)。"""
    import glob, importlib.util, os, sys
    want = os.environ.get("ENOMOTO_CORE", "auto")
    if want == "base":
        return
    here = os.path.dirname(os.path.abspath(__file__))
    cands = glob.glob(os.path.join(here, "_core_v3*.so"))
    if not cands:
        return
    if want != "v3":
        try:
            with open("/proc/cpuinfo") as f:
                flags = next((l.split(":", 1)[1].split() for l in f if l.startswith("flags")), [])
        except OSError:
            return
        if not {"avx", "avx2", "bmi1", "bmi2", "fma", "f16c", "movbe", "abm"} <= set(flags):
            return
    name = __name__ + "._core"
    spec = importlib.util.spec_from_file_location(name, cands[0])
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)


_enomoto_load_core()
'''
s = (o / "__init__.py").read_text()
k = s.index("from .constraint import")  # 他の部品 (model.py など) が _core を読むより前に
(o / "__init__.py").write_text(s[:k] + LOADER.lstrip("\n") + "\n" + s[k:])
print("ok", o)
