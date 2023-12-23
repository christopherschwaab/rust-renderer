#wayland_window:
#	g++ -o wayland_window -Og wayland_window.cpp -lwayland-client

all: p1

p%: p%.cpp
	g++ -o $@ $@.cpp -Og -lwayland-client

.PHONY: all
