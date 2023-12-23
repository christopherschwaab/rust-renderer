#!/usr/bin/env sh

nix-shell -p wayland --command "make -C . $*"
