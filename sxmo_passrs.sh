#!/bin/zsh
# title="passrs"
. sxmo_hook_icons.sh

export PASSRS_PASS=$(echo | sxmo_dmenu_with_kb.sh -x -p "Master password:" | tr -d "\n")
passrs_mode=$(echo "Passwords
Authenticator" | sxmo_dmenu.sh -p "Select mode")

case $passrs_mode in
    Passwords)
        if ! passwords="$(passrs -p)"; then
            exit 1
        fi
        sel_pass=$(echo $passwords | cut -d $'\t' -f 1 | sxmo_dmenu.sh -p "Select password")
        copy_pass=$(echo $passwords | grep "$sel_pass" | cut -d $'\t' -f 2-)
        ;;
    Authenticator)
        while true; do
            if ! totpcodes="$(passrs -t)"; then
                exit 1
            fi
            seconds="$icon_rld Refresh ($(($(date +%S) % 30))s)"
            sel_totp=$(echo "$seconds
$totpcodes" | sxmo_dmenu.sh -p "Select auth code")
            if [ "$sel_totp" != "$seconds" ]; then
                copy_pass=$(echo $sel_totp | cut -d $'\t' -f 2)
                break
            fi
        done
        ;;
esac

wl-copy "$copy_pass"
