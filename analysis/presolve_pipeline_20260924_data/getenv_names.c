#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define MAXN 512
static char names[MAXN][96]; static long counts[MAXN]; static int nn = 0;
static char *(*real_getenv)(const char *) = 0;
char *getenv(const char *name) {
    if (!real_getenv) real_getenv = dlsym(RTLD_NEXT, "getenv");
    int i; for (i = 0; i < nn; i++) if (strcmp(names[i], name) == 0) { counts[i]++; break; }
    if (i == nn && nn < MAXN) { strncpy(names[nn], name, 95); counts[nn] = 1; nn++; }
    return real_getenv(name);
}
__attribute__((destructor)) static void report(void) {
    FILE *f = fopen("/tmp/claude-0/-home-user-enomoto-solver/bf088fb6-b7b9-593b-9a3f-1a3d60f9da2a/scratchpad/ps/getenv_names.txt", "w");
    for (int i = 0; i < nn; i++) fprintf(f, "%ld %s\n", counts[i], names[i]); fclose(f);
}
