# 設定の名前 → 環境変数 (G=IMPLBD_GENINT、R=CUT_LP_RETRY=4、F=FACE_SEP=off、X=RESTART_REF)
G="ENOMOTO_MIP_IMPLBD_GENINT=1"; R="ENOMOTO_T_MIP_CUT_LP_RETRY=4"; F="ENOMOTO_MIP_FACE_SEP=off"; X="ENOMOTO_MIP_RESTART_REF=1"
cfg() {
  case $1 in
    def) echo "";;
    gfr) echo "$G $R $F $X";;
    noG) echo "$R $F $X";;
    noR) echo "$G $F $X";;
    noF) echo "$G $R $X";;
    noX) echo "$G $R $F";;
    G) echo "$G";;
    F) echo "$F";;
    GR) echo "$G $R";;
    R) echo "$R";;
    X) echo "$X";;
    *) echo "${!1}";;
  esac
}
