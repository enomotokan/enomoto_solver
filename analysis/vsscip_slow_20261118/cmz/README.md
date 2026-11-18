# 群 2 cmz: `ENOMOTO_MIP_CUTMIX=z` の広い計測 (2026-11-18)

試作 `ENOMOTO_MIP_CUTMIX=z` (src/mip/cutmix.rs の `zh_on`): 全部整数・係数の絶対値 2 以下の行の割合 `int_small` (外側の根で数える) が
`ENOMOTO_T_CUTMIX_ZH_MIN` (0.5) 以上の問題でだけ、既定で切ってある zero-half (`ENOMOTO_MIP_NO_ZEROHALF` の既定 `1`) を使う。
seymour で根 408.48 → 412.91 (SCIP 411.05)、600 秒の下界 412.0 → 416.0 (tree2/README.md 7 節)。58 問・Mittelmann では未計測 (analysis/cutmix_20261111.md)。

台本は domrev/ (slow2_20261117 の写し) の写し。設定は `cfg.py` (def・cmz)。

- M = Mittelmann 48 問 (vshighs の M) + seymour・hypothyroid-k1 × 種 1・2 × {def, cmz}、600 秒、**12 分担** (各 1.43 時間): `cd analysis/vsscip_slow_20261118/cmz && python run.py M K` (K = 0〜11)
- P = MIPLIB の 58 問 × 種 1〜3 × {def, cmz}、60 秒、**4 分担** (各 0.74 時間): `python run.py P K` (K = 0〜3)
- 集計: `python summ.py M cmz`・`python summ.py P cmz`。

## 働く問題の見当 (`intsmall.py` → `intsmall.txt`)

元の MPS で数えた int_small (本物は前処理の後で数えるので目安)。0.5 以上 (cmz で zero-half が入る見込み) は P 20 問・M 19 問:

- P: mzzv42z・neos-3083819-nubu・neos-911970・neos-960392・neos8・nu25-pr12・physiciansched6-2・roll3000・p0201・misc07・gt2・10teams・air03・fiber・
  eil33-2・nw04・blp-ar98・cvs16r128-89・mik-250-20-75-4・graph20-20-1rand
- M: enlight_hard・fast0507・fiball・irp・mcsched・mzzv11・mzzv42z・neos-3216931-puriri・neos-4722843-widden・neos-662469・neos-933966・neos-960392・
  rocI-4-11・rococoC10-001000・roll3000・satellites2-60-fs・sct2・seymour・tbfp-network

それ以外の問題では cmz は既定と同じ計算 (差は 4 スレッドのゆれ)。集計では、働く問題と働かない問題に分けて見る。

## P の結果 (4 分担、8507e53d5、`summ_split_P.txt`)

- 働く見込みの 20 問 (60 組): 解けた def 50 / cmz 47、cmz/def SGM **1.228**。10teams 5.44 倍 (5 → 17〜60 秒)・misc07 2.18・eil33-2 1.71
  (2 種で cmz だけ時間切れ)・nw04 1.43・nu25-pr12 1.32 と遅く、速いのは neos-3083819-nubu 0.91 だけ。
- 働かない 38 問 (114 組、既定と同じ計算): SGM 0.996、解けた 87 / 86 (揺れの幅の目安、noswot 1.70・mc11 0.74 など)。
- 結論: 全体の既定にはしない。seymour (setp 0.95・intsmall 1.0) と遅くなる nw04・air03 (集合分割、intsmall 1.0)・eil33-2 (1.0) を分ける特徴は
  intsmall・setp では見つからない。M (12 分担) は回さない (seymour だけを記録に残すなら seymour × 種 1・2 の 4 回で足りる)。
