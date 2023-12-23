#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <wayland-client.h>

wl_display *display = nullptr;
wl_compositor *compositor = nullptr;
wl_surface *surface = nullptr;
wl_shell *shell = nullptr;
wl_shell_surface *shell_surface = nullptr;
wl_shm *shm = nullptr;
wl_buffer *buffer = nullptr;
wl_callback *frame_callback = nullptr;

void *shm_data;

static const int WIDTH = 480;
static const int HEIGHT = 360;

static void shm_format(void *data, wl_shm *wl_shm, uint32_t format) {
	fprintf(stderr, "format %d\n", format);
}

wl_shm_listener shm_listener = {
	shm_format
};

static void global_registry_handler(void *data, wl_registry *reg, uint32_t id, const char *iface, uint32_t version) {
	printf("got a registry event for %s id %d\n", iface, id);
	if (strcmp(iface, "wl_compositor") == 0) {
		printf("  registering a wl_compositor_interface for id %d on iface %s (at verison 1)\n", id, iface);
		compositor = (wl_compositor*) wl_registry_bind(reg, id, &wl_compositor_interface, 1);
	} else if (strcmp(iface, "wl_shell") == 0) {
		shell = (wl_shell*) wl_registry_bind(reg, id, &wl_shell_interface, 1);
	} else if (strcmp(iface, "wl_shm") == 0) {
		shm = (wl_shm*) wl_registry_bind(reg, id, &wl_shm_interface, 1);
		wl_shm_add_listener(shm, &shm_listener, nullptr);
	}
}

static void global_registry_remover(void *data, wl_registry *reg, uint32_t id) {
	printf("got a registry losing event for %d\n", id);
}

static int create_tmpfile_cloexec(char *tmpname) {
	const int fd = mkostemp(tmpname, O_CLOEXEC);
	if (fd >= 0) {
		unlink(tmpname);
	}

	return fd;
}

int os_create_anonymous_file(off_t size) {
	static const char name_template[] = "/weston-shared-XXXXXX";
	char *name = nullptr;

	const char *path = getenv("XDG_RUNTIME_DIR");
	if (path == nullptr) {
		errno = ENOENT;
		return -1;
	}

	name = (char*) malloc(strlen(path) + sizeof(name_template));
	if (name == nullptr) {
		return -1;
	}

	strcpy(name, path);
	strcat(name, name_template);

	int fd = create_tmpfile_cloexec(name);
	free(name);

	if (fd < 0) {
		return -1;
	}
	if (ftruncate(fd, size) < 0) {
		close(fd);
		return -1;
	}

	return fd;
}

static wl_buffer *create_buffer() {
	const int stride = WIDTH * 4;
	const int size = stride * HEIGHT;

	wl_shm_pool *pool = nullptr;
	wl_buffer *buf = nullptr;

	const int fd = os_create_anonymous_file(size);
	if (fd < 0) {
		fprintf(stderr, "failed to create a buffer file for %d: %m\n", size);
		exit(1);
	}

	shm_data = mmap(nullptr, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
	if (shm_data == MAP_FAILED) {
		fprintf(stderr, "mmap failed: %m\n");
		close(fd);
		exit(1);
	}

	pool = wl_shm_create_pool(shm, fd, size);
	buf = wl_shm_pool_create_buffer(pool, 0, WIDTH, HEIGHT, stride, WL_SHM_FORMAT_XRGB8888);
	wl_shm_pool_destroy(pool);
	return buf;
}

static void handle_ping(void *data, wl_shell_surface *shell_surface, uint32_t serial) {
	wl_shell_surface_pong(shell_surface, serial);
	fprintf(stderr, "Pinged and ponged\n");
}

static void handle_configure(void *data, wl_shell_surface *shell_surface, uint32_t edges, int32_t width, int32_t height) {
}

static void handle_popup_done(void *data, wl_shell_surface *shell_surface) {
}

static const wl_shell_surface_listener shell_surface_listener = {
	handle_ping,
	handle_configure,
	handle_popup_done
};

uint32_t pixel_value = 0x000000;
static void paint() {
	uint32_t *pixel = (uint32_t*) shm_data;

	for (int n = 0; n < WIDTH * HEIGHT; ++n) {
		*pixel++ = pixel_value;
	}

	pixel_value += 0x10101;
	if (pixel_value > 0xffffff) {
		pixel_value = 0x0;
	}
}

static const struct wl_registry_listener registry_listener = {
	global_registry_handler,
	global_registry_remover,
};

static void create_window() {
	buffer = create_buffer();

	wl_surface_attach(surface, buffer, 0, 0);
	wl_surface_commit(surface);
}

static void redraw(void *data, wl_callback *callback, uint32_t time);

static const wl_callback_listener frame_listener = {
	redraw
};

static void redraw(void *data, wl_callback *callback, uint32_t time) {
	wl_callback_destroy(frame_callback);
	wl_surface_damage(surface, 0, 0, WIDTH, HEIGHT);
	paint();
	frame_callback = wl_surface_frame(surface);
	wl_surface_attach(surface, buffer, 0, 0);
	wl_callback_add_listener(frame_callback, &frame_listener, nullptr);
	wl_surface_commit(surface);
}

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

	surface = wl_compositor_create_surface(compositor);
	if (surface == nullptr) {
		fprintf(stderr, "failed to create surface\n");
		exit(1);
	} else {
		fprintf(stderr, "created surface\n");
	}

	if (shell == nullptr) {
		fprintf(stderr, "no wayland shell\n");
		exit(1);
	}

	shell_surface = wl_shell_get_shell_surface(shell, surface);
	if (shell_surface == nullptr) {
		fprintf(stderr, "failed to create shell surface\n");
		exit(1);
	} else {
		fprintf(stderr, "created shell surface\n");
	}
	wl_shell_surface_set_toplevel(shell_surface);
	wl_shell_surface_add_listener(shell_surface, &shell_surface_listener, nullptr);

	frame_callback = wl_surface_frame(surface);
	wl_callback_add_listener(frame_callback, &frame_listener, nullptr);

	create_window();
	redraw(nullptr, nullptr, 0);

	while (wl_display_dispatch(display) != -1) {
		;
	}

	wl_display_disconnect(display);
	printf("disconnected from display\n");

	exit(0);
}
