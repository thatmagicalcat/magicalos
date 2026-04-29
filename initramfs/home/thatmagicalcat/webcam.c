void *__dso_handle = 0;

#include <magicalos.h>

#include <fcntl.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>

// 4 MiB
#define SHM_SIZE (4 * 1024 * 1024)
#define MAGIC 0xCAFEBABE
#define PIXEL_BYTES 4

typedef struct ShmHeader {
  volatile uint32_t magic;
  volatile uint32_t width;
  volatile uint32_t height;

  // synchronization
  volatile uint32_t latest_frame_idx; // 0 or 1 (Updated by Host)
  volatile uint32_t guest_read_idx;   // 0 or 1 (Updated by Guest)
  volatile uint32_t frame_counter;    // Incremented by Host on new frame

  // two ping-pong buffers after the header...
} ShmHeader;

int main() {
  printf("Testing webcam\n");

  int video_fb = open("/dev/video0", O_RDWR);

  if (video_fb < 0) {
    perror("Failed to open /dev/video0");
    return 1;
  }

  printf("mmap shm\n");
  void *shm =
      mmap(NULL, SHM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, video_fb, 0);

  if (shm == MAP_FAILED) {
    perror("Failed to mmap /dev/video0");
    close(video_fb);
    return 1;
  }

  int fb_fd = open("/dev/fb0", O_RDWR);

  if (fb_fd < 0) {
    perror("Failed to open /dev/fb0");
    return 1;
  }

  FramebufferInfo fb_info;
  read(fb_fd, &fb_info, sizeof(fb_info));
  printf("Screen is %dx%d (%d bpp)\n", fb_info.width, fb_info.height,
         fb_info.bpp);

  printf("mmap framebuffer\n");
  size_t fb_size = fb_info.pitch * fb_info.height;
  uint32_t *fb_ptr =
      mmap(NULL, fb_size, PROT_READ | PROT_WRITE, MAP_SHARED, fb_fd, 0);

  if (fb_ptr == MAP_FAILED) {
    perror("Failed to mmap /dev/fb0");
    close(fb_fd);
    return 1;
  }

  // for (int y = 100; y < 200; y++)
  //   for (int x = 100; x < 200; x++)
  //     pixels[y * (info.pitch / 4) + x] = 0x00FF0000;

  ShmHeader *header = (ShmHeader *)shm;

  printf("Waiting for host to initialize the shared memory\n");
  while (header->magic != MAGIC)
    ; /* busy wait */
  printf("Host ready! Resolution %dx%d\n", header->width, header->height);

  size_t buffer_size = header->width * header->height * PIXEL_BYTES; // BGRA

  uint8_t *buffer0 = (uint8_t *)shm + sizeof(ShmHeader);
  uint8_t *buffer1 = buffer0 + buffer_size;

  uint32_t last_frame_counter = 0;

  for (;;) {
    // wait for new frame
    while (last_frame_counter == header->frame_counter)
      ; /* busy-wait */
    last_frame_counter = header->frame_counter;

    uint32_t target_frame = header->latest_frame_idx;
    header->guest_read_idx = target_frame; // Tell host we are using it

    uint8_t *target_input_buffer = target_frame ? buffer1 : buffer0;

    for (int y = 0; y < header->height; y++)
      memcpy((uint8_t *)fb_ptr + (y * fb_info.pitch),
             target_input_buffer + (y * header->width * PIXEL_BYTES),
             header->width * PIXEL_BYTES);

    target_frame = 1 - (target_frame & 1);
    header->guest_read_idx = target_frame;
  }

  close(video_fb);

  return 0;
}
