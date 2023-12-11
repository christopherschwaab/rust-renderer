#include <iostream>

#include <stdint.h>

#include <windows.h>

static uint32_t *fb = NULL;

static constexpr int INITIAL_WIDTH = 800;
static constexpr int INITIAL_HEIGHT = 600;

LRESULT CALLBACK WindowProc(HWND hwnd, UINT uMsg, WPARAM wParam, LPARAM lParam) {
    switch (uMsg) {
        case WM_CLOSE:
            DestroyWindow(hwnd);
            return LRESULT(0);

        case WM_DESTROY:
            PostQuitMessage(0);
            return LRESULT(0);

        case WM_PAINT:
            {
                BITMAPINFO bmi = {0};
                bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
                bmi.bmiHeader.biWidth = INITIAL_WIDTH;
                bmi.bmiHeader.biHeight = -INITIAL_HEIGHT;
                bmi.bmiHeader.biPlanes = 1;
                bmi.bmiHeader.biBitCount = 32;
                bmi.bmiHeader.biCompression = BI_RGB;

                PAINTSTRUCT ps;
                HDC hdc = BeginPaint(hwnd, &ps);
                StretchDIBits(
                    hdc,
                    0, 0, INITIAL_WIDTH, INITIAL_HEIGHT,
                    0, 0, INITIAL_WIDTH, INITIAL_HEIGHT,
                    fb,
                    &bmi,
                    DIB_RGB_COLORS,
                    SRCCOPY
                );
                EndPaint(hwnd, &ps);
                std::cout << "did paint" << std::endl;
            }
            return LRESULT(0);

        default:
            return DefWindowProc(hwnd, uMsg, wParam, lParam);
    }
}

int main(int argc, char *argv[]) {
    fb = new uint32_t[INITIAL_WIDTH * INITIAL_HEIGHT];
    // draw a 100x100 red square at 10,10 to 110,100
    for (int y = 10; y < 100; ++y) {
        for (int x = 10; x < 100; ++x) {
            fb[y * INITIAL_WIDTH + x] = 0xFF0000FF;
        }
    }

    for (int y = 150; y < 200; ++y) {
        for (int x = 150; x < 200; ++x) {
            fb[y * INITIAL_WIDTH + x] = 0x0000ffFF;
        }
    }

    HMODULE hInstance = GetModuleHandle(NULL);

    WNDCLASS wc = {0};
    wc.style = CS_HREDRAW | CS_VREDRAW | CS_OWNDC;
    wc.hInstance = hInstance;
    wc.lpszClassName = TEXT("internet");
    wc.lpfnWndProc = WindowProc;
    wc.cbWndExtra = sizeof(void*);
    if (!RegisterClass(&wc)) {
        std::cout << "RegisterClass failed" << std::endl;
        return 1;
    }

    // create the window
    HWND hwnd = CreateWindowEx(
        0,
        TEXT("internet"),
        TEXT("internet"),
        WS_VISIBLE | WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        INITIAL_WIDTH,
        INITIAL_HEIGHT,
        NULL,
        NULL,
        hInstance,
        NULL
    );
    if (!hwnd) {
        std::cout << "CreateWindowEx failed" << std::endl;
        return 1;
    }

    HDC hdc = GetDC(hwnd);
    BITMAPINFO bmi = {0};
    bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bmi.bmiHeader.biWidth = INITIAL_WIDTH;
    bmi.bmiHeader.biHeight = -INITIAL_HEIGHT;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    for (;;) {
        MSG msg;
        if (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE)) {
            if (msg.message == WM_QUIT) {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        }
        StretchDIBits(
            hdc,
            0, 0, INITIAL_WIDTH, INITIAL_HEIGHT,
            0, 0, INITIAL_WIDTH, INITIAL_HEIGHT,
            fb,
            &bmi,
            DIB_RGB_COLORS,
            SRCCOPY
        );
    }

    return 0;
}
