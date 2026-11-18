# LexRed (ENOMOTO_MIP_SYMDYN) を誤答なしで使える形にする (2026-11-18)

## 結論

- 2026-11-11〜11-18 の既定 `ENOMOTO_MIP_SYMDYN=f` (orbital fixing + LexRed) で出た誤答 (gate2-1 の旧版の p0201、fuzz_of.py の種 1293) の
  本当の原因は、**LexRed の生成元ごとの伝播の「等号を除く」締めの誤り** (src/mip/symdyn.rs の `DynSym::prop_one`)。orbital fixing との併用そのもの
  ではない (併用はこの不具合を踏む道を増やしていただけ)。
- 直し (6e0cf3bce): 順の位置 v で `x_v ≥ γ(x)_v` の等号が値 c でしか成り立たず、後ろの位置で等号が破れるなら等号を除く。等号を除くのは
  「`x_v ≥ c + 1` または `γ(x)_v ≤ c − 1`」の**どちらか一方**なのに、以前は**両方**を締めていた。x_v が分枝で c に固定されたノードでは
  `x_v ≥ c + 1` で矛盾し、最適解の代表を含むノードを刈っていた。直した形: 片側が c に固定されているときだけもう片側を締める
  (`x_v = c` なら `γ(x)_v ≤ c − 1`、`γ(x)_v = c` なら `x_v ≥ c + 1`)。`ENOMOTO_MIP_SYMDYN_STRICT_BOTH` で以前の動き。
- あわせて見つけた既存の問題 (0.1.9 から、値は正しく状態だけ): 目的の上限つきプロービング (OBJPROBE p) が根で「打ち切り値より良い解はない」を
  示して LP が実行不能になると、根のカットのループが失敗扱いで返って NotSolved で終えていた (fuzz_symdyn の種 1013・1082)。直し (c96ec3e54):
  失敗で返っても根の下界が打ち切り値に届いていれば最適で終える (7 か所)。
- SYMDYN の既定は今「使わない」(8f4a55ebd、orbital fixing だけ)。直した LexRed で f を既定に戻すかは、fuzz (下) と速さの計測で決める。

## 経緯と切り分け

1. p0201 (gate2-1 の 61d034858^ + TAB_HIGHS_SIG=1) の誤答は SYMDYN=f と fo で出て、0・1・1o・NO_SYM_OF では出なかった
   (analysis/wrong_widden_20261117 の 12 節)。併用が原因と見て orbital fixing を分枝の 0 側に絞った (3a496c0cb) が、fuzz_of の種 1293
   (支配集合、24 列、orbitope・カットなし) は f でも 1o (LexRed + 軌道の比べ) でも出続けた。
2. 種 1293 は SYMPRE=0 (前処理の疎な自己同型の探りを止める) で消える。探りで見つかる生成元は 24 列の巡回 1 個、探りなしは 3 個
   (反転を含む) で、どちらも独立の確かめ (`verify_gens.py`、問題の自己同型か) を通る。生成元の集まりが違うと道筋が変わるだけと判断。
3. 診断 `ENOMOTO_MIP_DEBUG_TRACE` を、デバッグ解そのものではなく「orbital fixing の生成元の閉包でのデバッグ解の像のどれか」を含むノードの
   追跡に広げた。種 1293 (1o) のノード 4 (順 σ = [5, 6]) で、symdyn の前は像 6 個が定義域の中、symdyn の後は 0 個。外した境界は
   「列 5 の下限 2」(0-1 列で矛盾) で、上の締めの誤り (x_5 が分枝で 1 に固定、等号を除くとき x_5 ≥ 2 を締めた) と分かった。
4. 直した後: 種 1293 は 1o・f・1 とも 10/10 正解 (以前の動き STRICT_BOTH では 1o 7/10・f 5/10 誤答)。旧版の p0201 は orbital fixing を
   元のまま (併用 f) でこの直しだけを入れて 0/40 (以前の動きでは 16/40)。orbital fixing の絞り込みは不要なので既定から外し、
   試作 `ENOMOTO_MIP_OF_BRANCH_ONLY` に残した。

## 確かめ (fuzz、`fuzz_lex.sh 名前 [環境変数]`)

`fuzz_of.py` (orbitope・カットなし 1000 問、orbitope なし 500 問、既定 500 問)、`fuzz_symdyn.py` (orbitope・カット・再スタートなし 400 問、既定 400 問)、
`mip_fuzz_symmetric.py` (orbitope なし 300 問) を 1・4 スレッドで。結果は `fuzz_<名前>.txt`。

(結果は下に足す)

## ファイル

- `fuzz_lex.sh`: 上の fuzz 群。`ofmps.py`: fuzz_of の 1 問を MPS に。`verify_gens.py`: DEBUG_DUMP_SYMGRAPH の生成元が問題の自己同型かを確かめる。
