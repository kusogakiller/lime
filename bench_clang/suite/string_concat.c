// Category G: String concatenation — equivalent to string_concat.lime
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
int main(void) {
    char *s = malloc(2);
    strcpy(s, "x");
    long long i = 0;
    while (i < 30000LL) {
        char *n = malloc(strlen(s) + 2);
        strcpy(n, s);
        strcat(n, "y");
        free(s);
        s = n;
        i = i + 1;
    }
    printf("%lld\n", (long long)strlen(s));
    free(s);
    return 0;
}
