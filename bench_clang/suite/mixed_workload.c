// Category J (realistic mixed workload) — equivalent to mixed_workload.lime.
// Tokenize + count frequencies via the same fixed vocabulary (ids 0..6).
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
static int word_id(const char *w) {
    if (strcmp(w, "lime") == 0) return 0;
    if (strcmp(w, "is") == 0) return 1;
    if (strcmp(w, "fast") == 0) return 2;
    if (strcmp(w, "safe") == 0) return 3;
    if (strcmp(w, "native") == 0) return 4;
    if (strcmp(w, "code") == 0) return 5;
    return 6;
}
int main(void) {
    char *text = _strdup("lime is fast safe native code ");
    long long reps = 0;
    while (reps < 2000) {
        char *nb = malloc(strlen(text) + 32);
        strcpy(nb, text); strcat(nb, "lime native code fast safe ");
        free(text); text = nb;
        reps = reps + 1;
    }
    long long counts[7] = {0,0,0,0,0,0,0};
    long long i = 0; char cur[256]; long long cl = 0;
    long long L = (long long)strlen(text);
    while (i < L) {
        unsigned char b = (unsigned char)text[i];
        if (b >= 0) {
            if (b == 32) {
                if (cl > 0) { cur[cl] = 0; counts[word_id(cur)]++; cl = 0; }
            } else { cur[cl] = (char)b; cl = cl + 1; }
        }
        i = i + 1;
    }
    if (cl > 0) { cur[cl] = 0; counts[word_id(cur)]++; }
    long long total = 0; long long k = 0;
    while (k < 7) { total = total + counts[k]; k = k + 1; }
    printf("%lld\n", total);
    printf("%lld\n", 7LL);
    return 0;
}
