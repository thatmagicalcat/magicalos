#include <stdio.h>

void *__dso_handle = 0;

int main(int argc, char *argv[], char *envp[]) {
    printf("Hello from the C standard library!\n\n");

    printf("Arguments (argc = %d):\n", argc);

    for (int i = 0; i < argc; i++) {
        printf("  argv[%d] = %s\n", i, argv[i]);
    }

    printf("\nEnvironment Variables:\n");
    printf("  envp[0] = %s\n", envp[0]);

    printf("\nEnter your name: ");
    fflush(stdout);

    char buffer[100];
    scanf("%99[^\n]", buffer);

    printf("Hello, %s!", buffer);

    printf("\nGoodbye!\n");
    return 0;
}
