#include <stdio.h>
#include <stdlib.h>

void *__dso_handle = 0;

int main(int argc, char *argv[], char *envp[]) {
    printf("Hello from MagicalOS!\n");

    printf("Arguments (argc = %d):\n", argc);

    for (int i = 0; i < argc; i++) {
        printf("  argv[%d] = %s\n", i, argv[i]);
    }

    printf("\nEnvironment Variables:\n");
    if (envp && envp[0]) {
        printf("  envp[0] = %s\n", envp[0]);
    } else {
        printf("  (None)\n");
    }

    printf("\nSuccessfully returning from main(). Goodbye!\n");

    return 0;
}
