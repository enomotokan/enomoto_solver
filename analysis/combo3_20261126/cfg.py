"""c2c の新しい版 c2c v3 (`c3`) の設定 (analysis/combo3_20261126、2026-11-26、親の依頼)。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド (run.py の TH)。
どれも既定 (先端: PAR4=fmr・ROOTPAR_CUT・PARIDLE・UNIFY_FIX が既定) に足す。既定と試作は同じ版で回す。

c3 = 今の c2c (analysis/combo2_20261125/cfg.py の C2C = C2 + SB_PROBEBD=1) に、次の部品を足したもの。部品は PARTS で 1 つずつ切り替えられる:
- `nrap`  : NODEROWS_ADAPT を r0k23 → r0k23p (`p` は φ の補正、analysis/nrashare_20261126 3 節で neos-911970 の害を消した形)。
- `share` : NRA_SHARE=1 (nra の統計を全スレッドの合計で数える。src/mip/solver.rs `nra_share_on`、0 でなければ有効。
            P の 4 スレッドで中立: analysis/nrashare_20261126 6 節。1 スレッドでは読まれない)。
- `enumt` : ENUM_ADAPT を 1s → 1st (`t`: 主でないスレッドの前処理に ENUM_ADAPT の時計を置く。src/mip/mod.rs `EnumClockGuard::new` が `t` を含むかで見る。
            analysis/nrashare_20261126 7 節。P の 4 スレッドで確かめ中)。
- `pskip` : SB_PROP_PSKIP=1 (伝播の境界つきで解いた強分枝の子の伸びを擬似費用に入れない。src/mip/solver.rs `sb_prop_pskip`、0 でなければ有効。
            analysis/c2c1t_20261126。P で確かめ中)。
`t`・`pskip` の P の結果が悪ければ、PARTS から外して (または c3_without で) 外した形を作る。"""
import importlib.util
from pathlib import Path
_s = importlib.util.spec_from_file_location("combo2_cfg", Path(__file__).resolve().parents[1] / "combo2_20261125" / "cfg.py")
_m = importlib.util.module_from_spec(_s); _s.loader.exec_module(_m)
C2C = dict(_m.C2C)
assert C2C["ENOMOTO_MIP_NODEROWS_ADAPT"] == "r0k23" and C2C["ENOMOTO_MIP_ENUM_ADAPT"] == "1s" and C2C["ENOMOTO_MIP_SB_PROBEBD"] == "1"

# 部品: 名前 → (環境変数, c3 での値)。c2c にある変数は値を置き換え、ない変数は足す
PARTS = {
    "nrap": ("ENOMOTO_MIP_NODEROWS_ADAPT", "r0k23p"),
    "share": ("ENOMOTO_MIP_NRA_SHARE", "1"),
    "enumt": ("ENOMOTO_MIP_ENUM_ADAPT", "1st"),
    "pskip": ("ENOMOTO_MIP_SB_PROP_PSKIP", "1"),
}
# c3 に入れる部品 (P の結果で外すときはここから消す)
USE = ["nrap", "share", "enumt", "pskip"]


def c3_without(*drop):
    """c2c に USE の部品のうち drop 以外を足した環境変数"""
    env = dict(C2C)
    for name in USE:
        if name not in drop:
            k, v = PARTS[name]; env[k] = v
    return env


C3 = c3_without()
P1 = {1: ("c3", C3, True, True, "combo3_20261126")}
CFGS = {"def_t1": {}, "def_t4": {}, "c3_t1": dict(C3), "c3_t4": dict(C3)}

if __name__ == "__main__":
    for k, v in sorted(C3.items()): print(f"{k}={v}")
