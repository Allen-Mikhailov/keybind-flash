kdialog --msgbox "Script Started!"
konsole ./


SCRIPT_DIR=$(dirname "$(readlink -f "$0")")


/usr/bin/python3 $SCRIPT_DIR/index.py
