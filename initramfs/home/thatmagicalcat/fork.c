void *__dso_handle = 0;

#include <stdio.h>
#include <unistd.h>

int main() {
    printf("Fork!\n");

    pid_t pid = fork();

    if (pid == 0) printf("I'm the child process!\n");
    else printf("I'm the parent process, child PID: %u\n", pid);

    return 0;
}
