#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <fcntl.h>
#include <time.h>
#include <unistd.h>
#include <sys/mman.h>
#include <wayland-client-core.h>
#include <wayland-client-protocol.h>
#include <wayland-client.h>

extern "C" {
#include "xdg-shell.h"
}

wl_compositor *wl_compositor = nullptr;
xdg_wm_base *xdg_wm_base = nullptr;
wl_shm *wl_shm = nullptr;

int open_shm_file(const size_t size) {
    wl_buffer *wl_buffer =  nullptr;

    struct timespec ts;
    char *filename = nullptr;
    if (asprintf(&filename, "/xdg_wm2_%d_%d", getpid(), clock_gettime(CLOCK_MONOTONIC, &ts)) == -1) {
        fprintf(stderr, "failed to create shm filename: %m\n");
        return -1;
    }
    const int fd = shm_open(filename, O_RDWR | O_CREAT | O_EXCL, S_IRUSR | S_IWUSR);
    if (fd < 0) {
        fprintf(stderr, "failed to create shm file: %m\n");
        goto cleanup;
    }
    if (ftruncate(fd, size) < 0) {
        close(fd);
    }

cleanup:
    shm_unlink(filename);
    free(filename);
    return fd;
}

struct shm_pixel_buffer {
    struct wl_buffer *wl_buffer;
    struct wl_shm_pool *wl_shm_pool;
    uint8_t *pool_buffer;
    int width;
    int height;
    int fd;
};

bool alloc_shm_pixel_buffer(int width, int height, shm_pixel_buffer *out_buffer) {
    bool ok = false;

    wl_buffer *wl_buffer =  nullptr;
    const int bytes_per_pixel = 4;
    const size_t size = width * height * bytes_per_pixel;

    int fd = open_shm_file(size);
    if (fd < 0) {
        return ok;
    }
    uint8_t *pool_buffer = static_cast<uint8_t *>(mmap(nullptr, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0));
    wl_shm_pool *wl_shm_pool = nullptr;
    if (pool_buffer == MAP_FAILED) {
        fprintf(stderr, "mmap failed: %m\n");
        goto cleanup;
    }

    wl_shm_pool = wl_shm_create_pool(wl_shm, fd, size);
    if (wl_shm_pool == nullptr) {
        goto cleanup;
    }
    wl_buffer = wl_shm_pool_create_buffer(wl_shm_pool, 0, width, height, width * bytes_per_pixel, WL_SHM_FORMAT_XRGB8888);

    *out_buffer = {
        wl_buffer,
        wl_shm_pool,
        pool_buffer,
        width,
        height,
        fd,
    };
    ok = true;

cleanup:
    close(fd);
    return ok;
}

static void handle_notify_global(void *data, wl_registry *wl_registry, uint32_t name, const char *interface, uint32_t version) {
    printf("got a registry event for %s id %d\n", interface, name);
    if (strcmp(interface, "wl_compositor") == 0) {
        wl_compositor = static_cast<struct wl_compositor *>(wl_registry_bind(wl_registry, name, &wl_compositor_interface, 1));
    } else if (strcmp(interface, "xdg_wm_base") == 0) {
        xdg_wm_base = static_cast<struct xdg_wm_base *>(wl_registry_bind(wl_registry, name, &xdg_wm_base_interface, 1));
    } else if (strcmp(interface, "wl_shm") == 0) {
        wl_shm = static_cast<struct wl_shm *>(wl_registry_bind(wl_registry, name, &wl_shm_interface, 1));
    }
}

static void handle_configure(void *data, struct xdg_surface *xdg_surface, uint32_t serial) {
    printf("got configure\n");
    xdg_surface_ack_configure(xdg_surface, serial);
}

static const wl_registry_listener wl_registry_listener = {
  handle_notify_global,
  nullptr
};

static const xdg_surface_listener xdg_surface_listener = {
  handle_configure
};

void draw(shm_pixel_buffer *buf) {
    uint32_t *data = (uint32_t *) buf->pool_buffer;
    for (int y = 0; y < buf->height; ++y) {
        for (int x = 0; x < buf->width; ++x) {
          if ((x + y / 8 * 8) % 16 < 8) {
            data[y * buf->width + x] = 0xFF666666;
          } else {
            data[y * buf->width + x] = 0xFFEEEEEE;
          }
        }
    }
}

int main(int argc, char **argv) {
    wl_display *display = wl_display_connect(nullptr);
    if (display == nullptr) {
      fprintf(stderr, "failed to connect to display\n");
      exit(1);
    }

    wl_registry *registry = wl_display_get_registry(display);
    wl_registry_add_listener(registry, &wl_registry_listener, nullptr);

    wl_display_roundtrip(display);
    wl_display_dispatch(display);

    wl_surface *wl_surface = wl_compositor_create_surface(wl_compositor);
    if (wl_surface == nullptr) {
      fprintf(stderr, "failed to create surface\n");
      exit(1);
    }

    xdg_surface *xdg_surface = xdg_wm_base_get_xdg_surface(xdg_wm_base, wl_surface);
    if (xdg_surface == nullptr) {
        fprintf(stderr, "failed to create xdg_surface\n");
        exit(1);
    }

    xdg_toplevel *xdg_toplevel = xdg_surface_get_toplevel(xdg_surface);
    if (xdg_toplevel == nullptr) {
        fprintf(stderr, "failed to create xdg_toplevel\n");
        exit(1);
    }

    xdg_surface_add_listener(xdg_surface, &xdg_surface_listener, nullptr);

    shm_pixel_buffer shm_pixel_buffer = {};
    const int width = 480;
    const int height = 360;
    if (!alloc_shm_pixel_buffer(width, height, &shm_pixel_buffer)) {
        fprintf(stderr, "failed to allocate shm pixel buffer\n");
        exit(1);
    }

    draw(&shm_pixel_buffer);
    wl_surface_attach(wl_surface, shm_pixel_buffer.wl_buffer, 0, 0);
    wl_surface_commit(wl_surface);
    while (wl_display_dispatch(display) != -1) {
        ;
    }

    wl_display_disconnect(display);

    return 0;
}
