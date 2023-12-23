#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wayland-client.h>

wl_display *display = nullptr;
wl_compositor *compositor = nullptr;

static void global_registry_handler(void *data, wl_registry *reg, uint32_t id, const char *iface, uint32_t version) {
	printf("got a registry event for %s id %d\n", iface, id);
	if (strcmp(iface, "wl_compositor") == 0) {
		printf("  registering a wl_compositor_interface for id %d on iface %s (at verison 1)\n", id, iface);
		compositor = (wl_compositor*) wl_registry_bind(reg, id, &wl_compositor_interface, 1);
	}
}

static void global_registry_remover(void *data, wl_registry *reg, uint32_t id) {
	printf("got a registry losing event for %d\n", id);
}

static const struct wl_registry_listener registry_listener = {
	global_registry_handler,
	global_registry_remover,
};

int main(int argc, char **argv) {
	display = wl_display_connect(nullptr);
	if (display == nullptr) {
		fprintf(stderr, "failed to connect to display\n");
		exit(1);
	}
	printf("connected to display\n");

	wl_registry *registry = wl_display_get_registry(display);
	wl_registry_add_listener(registry, &registry_listener, nullptr);

	wl_display_dispatch(display);
	wl_display_roundtrip(display);

	if (compositor == nullptr) {
		fprintf(stderr, "failed to find compositor\n");
		exit(1);
	} else {
		fprintf(stderr, "found compositor\n");
	}

	wl_display_disconnect(display);
	printf("disconnected from display\n");

	exit(0);
}
