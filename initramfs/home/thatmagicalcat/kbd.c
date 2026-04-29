#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <unistd.h>

void *__dso_handle = 0;

struct RawKeyEvent {
    uint64_t timestamp_nanos;
    uint8_t code;
    uint8_t state;
};

int main() {
  printf("testing keyboard device\n");

  int fd = open("/dev/kbd", O_RDONLY);
  struct RawKeyEvent event;

  while (read(fd, &event, sizeof(event)))
    printf("key %s, code: %d, timestamp %lu ns\n", event.state ? "down" : "up", event.code, event.timestamp_nanos);

  return 0;
}
