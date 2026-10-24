// RemoteDesk Free - Windows Input Injection Implementation
// Copyright (C) 2026 Suresh Mamidala | Freeware

#define REMOTEDESK_NATIVE_EXPORTS
#include <windows.h>
#include "remotedesk_native_api.h"

extern "C" {

bool rd_input_mouse_move(int32_t x, int32_t y, uint32_t screen_w, uint32_t screen_h) {
    if (screen_w == 0 || screen_h == 0) {
        screen_w = GetSystemMetrics(SM_CXSCREEN);
        screen_h = GetSystemMetrics(SM_CYSCREEN);
    }

    INPUT input = {};
    input.type = INPUT_MOUSE;
    // Map to normalized absolute desktop coordinates (0 to 65535)
    input.mi.dx = (LONG)((x * 65535.0f) / screen_w);
    input.mi.dy = (LONG)((y * 65535.0f) / screen_h);
    input.mi.dwFlags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE;

    return SendInput(1, &input, sizeof(INPUT)) == 1;
}

bool rd_input_mouse_down(RdMouseButton button, int32_t x, int32_t y, uint32_t screen_w, uint32_t screen_h) {
    rd_input_mouse_move(x, y, screen_w, screen_h);

    INPUT input = {};
    input.type = INPUT_MOUSE;

    switch (button) {
        case RD_MOUSE_LEFT:   input.mi.dwFlags = MOUSEEVENTF_LEFTDOWN; break;
        case RD_MOUSE_RIGHT:  input.mi.dwFlags = MOUSEEVENTF_RIGHTDOWN; break;
        case RD_MOUSE_MIDDLE: input.mi.dwFlags = MOUSEEVENTF_MIDDLEDOWN; break;
        case RD_MOUSE_X1:
            input.mi.dwFlags = MOUSEEVENTF_XDOWN;
            input.mi.mouseData = XBUTTON1;
            break;
        case RD_MOUSE_X2:
            input.mi.dwFlags = MOUSEEVENTF_XDOWN;
            input.mi.mouseData = XBUTTON2;
            break;
        default: return false;
    }

    return SendInput(1, &input, sizeof(INPUT)) == 1;
}

bool rd_input_mouse_up(RdMouseButton button, int32_t x, int32_t y, uint32_t screen_w, uint32_t screen_h) {
    rd_input_mouse_move(x, y, screen_w, screen_h);

    INPUT input = {};
    input.type = INPUT_MOUSE;

    switch (button) {
        case RD_MOUSE_LEFT:   input.mi.dwFlags = MOUSEEVENTF_LEFTUP; break;
        case RD_MOUSE_RIGHT:  input.mi.dwFlags = MOUSEEVENTF_RIGHTUP; break;
        case RD_MOUSE_MIDDLE: input.mi.dwFlags = MOUSEEVENTF_MIDDLEUP; break;
        case RD_MOUSE_X1:
            input.mi.dwFlags = MOUSEEVENTF_XUP;
            input.mi.mouseData = XBUTTON1;
            break;
        case RD_MOUSE_X2:
            input.mi.dwFlags = MOUSEEVENTF_XUP;
            input.mi.mouseData = XBUTTON2;
            break;
        default: return false;
    }

    return SendInput(1, &input, sizeof(INPUT)) == 1;
}

bool rd_input_mouse_wheel(int32_t delta_x, int32_t delta_y) {
    INPUT input = {};
    input.type = INPUT_MOUSE;

    if (delta_y != 0) {
        input.mi.dwFlags = MOUSEEVENTF_WHEEL;
        input.mi.mouseData = (DWORD)delta_y;
        SendInput(1, &input, sizeof(INPUT));
    }
    if (delta_x != 0) {
        input.mi.dwFlags = MOUSEEVENTF_HWHEEL;
        input.mi.mouseData = (DWORD)delta_x;
        SendInput(1, &input, sizeof(INPUT));
    }

    return true;
}

bool rd_input_key_down(uint32_t vk_code, uint32_t scan_code, bool extended) {
    INPUT input = {};
    input.type = INPUT_KEYBOARD;
    input.ki.wVk = (WORD)vk_code;
    input.ki.wScan = (WORD)(scan_code > 0 ? scan_code : MapVirtualKeyW(vk_code, MAPVK_VK_TO_VSC));
    input.ki.dwFlags = extended ? KEYEVENTF_EXTENDEDKEY : 0;

    return SendInput(1, &input, sizeof(INPUT)) == 1;
}

bool rd_input_key_up(uint32_t vk_code, uint32_t scan_code, bool extended) {
    INPUT input = {};
    input.type = INPUT_KEYBOARD;
    input.ki.wVk = (WORD)vk_code;
    input.ki.wScan = (WORD)(scan_code > 0 ? scan_code : MapVirtualKeyW(vk_code, MAPVK_VK_TO_VSC));
    input.ki.dwFlags = KEYEVENTF_KEYUP | (extended ? KEYEVENTF_EXTENDEDKEY : 0);

    return SendInput(1, &input, sizeof(INPUT)) == 1;
}

bool rd_input_text_utf16(const wchar_t* text) {
    if (!text) return false;

    while (*text) {
        INPUT inputs[2] = {};
        inputs[0].type = INPUT_KEYBOARD;
        inputs[0].ki.wScan = (WORD)*text;
        inputs[0].ki.dwFlags = KEYEVENTF_UNICODE;

        inputs[1].type = INPUT_KEYBOARD;
        inputs[1].ki.wScan = (WORD)*text;
        inputs[1].ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;

        SendInput(2, inputs, sizeof(INPUT));
        text++;
    }

    return true;
}

} // extern "C"
