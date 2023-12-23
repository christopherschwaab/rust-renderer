#wayland_window:
#	g++ -o wayland_window -Og wayland_window.cpp -lwayland-client

CFLAGS := -Og
CPPFLAGS := $(CFLAGS)

all: p3 xdg_wm

p%: p%.cpp
	g++ -o $@ $@.cpp $(CPPFLAGS) -lwayland-client

xdg_wm: xdg_wm.cpp xdg-shell.h xdg-shell.o
	g++ -o $@ $(filter-out $(wildcard *.h), $^) $(CPPFLAGS) -lwayland-client

xdg-shell.o: xdg-shell.c
	gcc -c -o $@ $< $(CFLAGS)

xdg-shell.h:
	wayland-scanner client-header xdg-shell-1.32.xml xdg-shell.h

xdg-shell.c:
	wayland-scanner private-code xdg-shell-1.32.xml xdg-shell.c

main.exe:
	x86_64-w64-mingw32-g++ main.cpp -o main.exe \
	  -D UNICODE -D _UNICODE \
	  -lgdi32 \
	  -static-libgcc \
	  -static-libstdc++ \
	  -static \
	  -lpthread

.PHONY: all
