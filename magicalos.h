#pragma once

#ifndef MAGICALOS_H
#define MAGICALOS_H

#include <stdint.h>

typedef struct FramebufferInfo {
    uint32_t width;
    uint32_t height;
    uint32_t pitch;
    uint16_t bpp;
    uint8_t memory_model;
    uint8_t r_sz, r_shift, g_sz, g_shift, b_sz, b_shift;
} FramebufferInfo;

typedef struct RawKeyEvent {
    uint64_t timestamp_nanos;
    uint8_t code;
    uint8_t state;
} RawKeyEvent;

#endif
