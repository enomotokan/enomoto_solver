# 設定の名前 → 環境変数 (G=IMPLBD_GENINT、R=CUT_LP_RETRY=4、F=FACE_SEP=off、X=RESTART_REF)
G="ENOMOTO_MIP_IMPLBD_GENINT=1"; R="ENOMOTO_T_MIP_CUT_LP_RETRY=4"; F="ENOMOTO_MIP_FACE_SEP=off"; X="ENOMOTO_MIP_RESTART_REF=1"
QF="ENOMOTO_MIP_GR_AUTO=2 ENOMOTO_MIP_FACE_SEP_FC=off $X"
cfg() {
  case $1 in
    def) echo "";;
    Qf) echo "$QF";;
    QfnoX) echo "ENOMOTO_MIP_GR_AUTO=2 ENOMOTO_MIP_FACE_SEP_FC=off";;
    X) echo "$X";;
    G) echo "$G";; R) echo "$R";; F) echo "$F";;
    GR) echo "$G $R";; GF) echo "$G $F";; RF) echo "$R $F";; GRF) echo "$G $R $F";;
    FX) echo "$F $X";;
    *) echo "${!1}";;
  esac
}
