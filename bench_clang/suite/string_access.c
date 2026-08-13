// Category G: String access — equivalent to string_access.lime (no free of literal).
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
int main(void) {
    char *s = malloc(1);
    strcpy(s, "");
    long long i = 0;
    while (i < 5000LL) {
        char *n = malloc(strlen(s) + 5);
        strcpy(n, s);
        strcat(n, "Lime");
        free(s);
        s = n;
        i = i + 1;
    }
    long long total = 0;
    long long j = 0;
    long long L = (long long)strlen(s);
    while (j < L) {
        unsigned char b = (unsigned char)s[j];
        if (b >= 0) {
            total = total + 1;
        }
        j = j + 1;
    }
    printf("%lld\n", total);
    free(s);
    return 0;
}
