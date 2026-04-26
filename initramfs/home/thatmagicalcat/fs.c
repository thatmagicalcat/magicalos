#include <string.h>
#include <unistd.h>
#include <fcntl.h>

void *__dso_handle = 0;

int main() {
    int fd = open("message.txt", O_CREAT | O_WRONLY);
    const char *msg = "The quick brown fox jumps over the lazy dog.\n";
    int len = write(fd, msg, strlen(msg));
    close(fd);

    return 0;
}
