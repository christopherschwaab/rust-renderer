#!/usr/bin/env sh

x86_64-w64-mingw32-g++ main.cpp -o main.exe \
    -D UNICODE -D _UNICODE \
    -lgdi32 \
    -static-libgcc \
    -static-libstdc++ \
    -static \
    -lpthread
