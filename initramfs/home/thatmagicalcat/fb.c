#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <stdint.h>
#include <stdio.h>

void *__dso_handle = 0;

struct fb_info {
    uint32_t width;
    uint32_t height;
    uint32_t pitch;
    uint16_t bpp;
    uint8_t memory_model;
    uint8_t r_sz, r_shift, g_sz, g_shift, b_sz, b_shift;
};

int main() {
    int fd = open("/dev/fb0", O_RDWR);
    
    struct fb_info info;
    read(fd, &info, sizeof(info));
    printf("Screen is %dx%d (%d bpp)\n", info.width, info.height, info.bpp);
    
    size_t fb_size = info.pitch * info.height;
    uint32_t* pixels = (uint32_t*)mmap(NULL, fb_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    
    for (int y = 100; y < 200; y++) {
        for (int x = 100; x < 200; x++) {
            pixels[y * (info.pitch / 4) + x] = 0x00FF0000; 
        }
    }
    
    return 0;
}
