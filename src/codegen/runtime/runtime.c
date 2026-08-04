// Lime Runtime Library
//
// Embedded in the compiler (src/codegen/runtime/) and compiled on-the-fly when
// linking compiled Lime programs into executables. The behaviour (including
// error messages and edge cases) matches the target runtime object
// (test_print/runtime.obj) so that programs behave identically.

#define _CRT_SECURE_NO_WARNINGS

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include <math.h>
#include <stdint.h>
#include "runtime.h"

#ifdef _WIN32
#include <windows.h>
#include <io.h>
#include <direct.h>
#include <sys/stat.h>
#else
#include <unistd.h>
#include <dirent.h>
#include <time.h>
#include <sys/stat.h>
#endif

// -- Allocation / control flow --

// Allocate `size` bytes (rounded up so zero-size requests get at least 1
// byte). Aborts with a panic message on out-of-memory.
void* runtime_alloc(int64_t size, int64_t align) {
    (void)align;
    if (size < 2) size = 1;
    void* p = malloc((size_t)size);
    if (p == NULL) {
        runtime_panic("runtime_alloc: out of memory");
    }
    return p;
}

void runtime_free(void* p) {
    free(p);
}

// Print an error to stderr and abort. `msg == NULL` is displayed as "(null)".
void runtime_panic(char* msg) {
    if (msg == NULL) msg = "(null)";
    fprintf(stderr, "Lime runtime panic: %s\n", msg);
    abort();
}

// -- Print --
// Print a NUL-terminated string to stdout. NULL is a no-op.
void runtime_print(char* s) {
    if (s == NULL) return;
    fputs(s, stdout);
}

// -- String operations --

// Substring [start, end) using byte offsets, clamped to the string bounds.
// NULL input yields an empty string.
char* runtime_str_slice(char* s, int64_t start, int64_t end) {
    if (s == NULL) {
        char* r = (char*)malloc(1);
        if (r != NULL) r[0] = '\0';
        return r;
    }
    int64_t len = (int64_t)strlen(s);
    int64_t lo = start < 0 ? 0 : start;
    int64_t hi = end < len ? end : len;
    if (hi < lo) hi = lo;
    int64_t n = hi - lo;
    char* r = (char*)malloc((size_t)(n + 1));
    if (r == NULL) {
        runtime_panic("runtime_str_slice: out of memory");
    }
    memcpy(r, s + lo, (size_t)n);
    r[n] = '\0';
    return r;
}

// Concatenate two NUL-terminated strings. NULL operands are treated as "".
char* runtime_str_concat(char* a, char* b) {
    if (a == NULL) a = "";
    if (b == NULL) b = "";
    size_t la = strlen(a);
    size_t lb = strlen(b);
    char* r = (char*)malloc(la + lb + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_concat: out of memory");
    }
    memcpy(r, a, la);
    memcpy(r + la, b, lb);
    r[la + lb] = '\0';
    return r;
}

// -- stdlib runtime helpers (Phase 12 Step 1) --
// These back the `string`/`math`/`time`/`fs`/`io` stdlib package builtins when
// a program is compiled natively. Behaviour (byte-length semantics, edge cases)
// mirrors the interpreter builtins in src/lib.rs.

static char* runtime_str_copy(const char* s) {
    if (s == NULL) s = "";
    size_t n = strlen(s);
    char* r = (char*)malloc(n + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_copy: out of memory");
    }
    memcpy(r, s, n + 1);
    return r;
}

// -- str() conversion helpers (Phase B-1) --
// Back the `str(...)` builtin for the primitive types in native codegen,
// mirroring `Value::to_string()` in the interpreter.

char* runtime_str_from_i64(int64_t v) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%lld", (long long)v);
    return runtime_str_copy(buf);
}

char* runtime_str_from_f64(double v) {
    // `%g` matches the native `printf("%g")` path used by `println(Float)`,
    // so `str(x)` and `println(x)` render identically for every float. In the
    // interpreter both paths also agree (`Value::to_string` -> `f64::to_string`),
    // so integer-valued results (as produced by math.floor/ceil/round) print the
    // same ("1", "-2") in both engines. Precision of non-integral floats is a
    // documented limitation: native uses %g (6 significant digits) while the
    // interpreter uses Rust's shortest round-trip repr.
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", v);
    return runtime_str_copy(buf);
}

char* runtime_str_from_bool(int8_t v) {
    return runtime_str_copy(v ? "true" : "false");
}

int runtime_str_contains(char* s, char* sub) {
    if (s == NULL) s = "";
    if (sub == NULL) sub = "";
    return strstr(s, sub) != NULL;
}

int runtime_str_starts_with(char* s, char* prefix) {
    if (s == NULL) s = "";
    if (prefix == NULL) prefix = "";
    size_t n = strlen(prefix);
    return strncmp(s, prefix, n) == 0;
}

int runtime_str_ends_with(char* s, char* suffix) {
    if (s == NULL) s = "";
    if (suffix == NULL) suffix = "";
    size_t slen = strlen(s);
    size_t n = strlen(suffix);
    if (n > slen) return 0;
    return strcmp(s + slen - n, suffix) == 0;
}

// Trim ASCII whitespace from both ends (mirrors the interpreter's Unicode
// `str::trim` for the ASCII subset).
char* runtime_str_trim(char* s) {
    if (s == NULL) return runtime_str_copy("");
    char* start = s;
    while (*start && isspace((unsigned char)*start)) start++;
    char* end = start + strlen(start);
    while (end > start && isspace((unsigned char)end[-1])) end--;
    size_t n = (size_t)(end - start);
    char* r = (char*)malloc(n + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_trim: out of memory");
    }
    memcpy(r, start, n);
    r[n] = '\0';
    return r;
}

// Replace every occurrence of `from` with `to` (Rust `str::replace` for the
// ASCII subset; replacing an empty `from` returns a copy of `s`).
char* runtime_str_replace(char* s, char* from, char* to) {
    if (s == NULL) s = "";
    if (from == NULL || *from == '\0') return runtime_str_copy(s);
    if (to == NULL) to = "";
    size_t len = strlen(s);
    size_t flen = strlen(from);
    size_t tlen = strlen(to);

    size_t count = 0;
    const char* p = s;
    while ((p = strstr(p, from)) != NULL) {
        count++;
        p += flen;
    }

    char* r = (char*)malloc(len + count * (tlen - flen) + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_replace: out of memory");
    }
    char* w = r;
    const char* cur = s;
    while ((p = strstr(cur, from)) != NULL) {
        size_t n = (size_t)(p - cur);
        memcpy(w, cur, n);
        w += n;
        memcpy(w, to, tlen);
        w += tlen;
        cur = p + flen;
    }
    size_t rest = strlen(cur);
    memcpy(w, cur, rest);
    w += rest;
    *w = '\0';
    return r;
}

// Split `s` on every occurrence of `sep` (Rust `str::split` semantics: an empty
// string yields one empty piece; trailing separators yield a trailing empty
// piece). The pieces are malloc'd strings stored as i64 slots in the list.
LimeList runtime_str_split(char* s, char* sep) {
    LimeList list = runtime_list_empty();
    if (s == NULL) return list;
    if (sep == NULL || *sep == '\0') {
        // Rust: "abc".split("") == ["a", "b", "c"]. We split on each byte
        // (matches for ASCII; multi-byte UTF-8 is not split correctly).
        size_t n = strlen(s);
        for (size_t i = 0; i < n; i++) {
            char* part = (char*)malloc(2);
            if (part == NULL) {
                runtime_panic("runtime_str_split: out of memory");
            }
            part[0] = s[i];
            part[1] = '\0';
            list = runtime_list_add(list, (int64_t)(intptr_t)part);
        }
        return list;
    }
    size_t flen = strlen(sep);
    const char* cur = s;
    for (;;) {
        const char* hit = strstr(cur, sep);
        size_t piece_len = hit != NULL ? (size_t)(hit - cur) : strlen(cur);
        char* part = (char*)malloc(piece_len + 1);
        if (part == NULL) {
            runtime_panic("runtime_str_split: out of memory");
        }
        memcpy(part, cur, piece_len);
        part[piece_len] = '\0';
        list = runtime_list_add(list, (int64_t)(intptr_t)part);
        if (hit == NULL) break;
        cur = hit + flen;
    }
    return list;
}

char* runtime_str_to_upper(char* s) {
    if (s == NULL) return runtime_str_copy("");
    size_t n = strlen(s);
    char* r = (char*)malloc(n + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_to_upper: out of memory");
    }
    for (size_t i = 0; i < n; i++) {
        r[i] = (char)toupper((unsigned char)s[i]);
    }
    r[n] = '\0';
    return r;
}

char* runtime_str_to_lower(char* s) {
    if (s == NULL) return runtime_str_copy("");
    size_t n = strlen(s);
    char* r = (char*)malloc(n + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_to_lower: out of memory");
    }
    for (size_t i = 0; i < n; i++) {
        r[i] = (char)tolower((unsigned char)s[i]);
    }
    r[n] = '\0';
    return r;
}

char* runtime_str_repeat(char* s, int64_t times) {
    if (s == NULL) s = "";
    if (times < 0) times = 0;
    size_t n = strlen(s);
    if (n != 0 && (uint64_t)times > (SIZE_MAX - 1) / n) {
        runtime_panic("runtime_str_repeat: overflow");
    }
    size_t total = (size_t)times * n;
    char* r = (char*)malloc(total + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_repeat: out of memory");
    }
    char* w = r;
    for (int64_t i = 0; i < times; i++) {
        memcpy(w, s, n);
        w += n;
    }
    *w = '\0';
    return r;
}

// Whether the string is empty.
int runtime_str_is_empty(char* s) {
    if (s == NULL) return 1;
    return s[0] == '\0';
}

// Find the first occurrence of `sub` in `s`. Returns the byte offset,
// or -1 if not found.
int64_t runtime_str_find(char* s, char* sub) {
    if (s == NULL) s = "";
    if (sub == NULL) sub = "";
    char* p = strstr(s, sub);
    if (p == NULL) return -1;
    return (int64_t)(p - s);
}

// Count non-overlapping occurrences of `sub` in `s`.
int64_t runtime_str_count(char* s, char* sub) {
    if (s == NULL) s = "";
    if (sub == NULL || *sub == '\0') return 0;
    int64_t count = 0;
    const char* p = s;
    size_t flen = strlen(sub);
    while ((p = strstr(p, sub)) != NULL) {
        count++;
        p += flen;
    }
    return count;
}

// Trim ASCII whitespace from the start (mirrors the interpreter's
// Unicode str::trim_start for the ASCII subset).
char* runtime_str_trim_start(char* s) {
    if (s == NULL) return runtime_str_copy("");
    char* start = s;
    while (*start && isspace((unsigned char)*start)) start++;
    return runtime_str_copy(start);
}

// Trim ASCII whitespace from the end.
char* runtime_str_trim_end(char* s) {
    if (s == NULL) return runtime_str_copy("");
    size_t n = strlen(s);
    while (n > 0 && isspace((unsigned char)s[n - 1])) n--;
    char* r = (char*)malloc(n + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_trim_end: out of memory");
    }
    memcpy(r, s, n);
    r[n] = '\0';
    return r;
}

// Join a List(str) with `sep` between each element.
char* runtime_str_join(LimeList* list, char* sep) {
    if (sep == NULL) sep = "";
    size_t seplen = strlen(sep);
    // Compute total length.
    size_t total = 0;
    int64_t count = runtime_list_len(*list);
    for (int64_t i = 0; i < count; i++) {
        char* item = (char*)(intptr_t)runtime_list_get(*list, i);
        if (item == NULL) item = "";
        total += strlen(item);
    }
    total += seplen * (count > 0 ? (size_t)(count - 1) : 0);
    char* r = (char*)malloc(total + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_join: out of memory");
    }
    char* w = r;
    for (int64_t i = 0; i < count; i++) {
        char* item = (char*)(intptr_t)runtime_list_get(*list, i);
        if (item == NULL) item = "";
        size_t ilen = strlen(item);
        memcpy(w, item, ilen);
        w += ilen;
        if (i < count - 1) {
            memcpy(w, sep, seplen);
            w += seplen;
        }
    }
    *w = '\0';
    return r;
}

// Parse s as a signed integer. Returns 0 on failure.
int64_t runtime_str_to_int(char* s) {
    if (s == NULL || *s == '\0') return 0;
    char* end;
    long long val = strtoll(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (int64_t)val;
}

// Parse s as a float. Returns 0.0 on failure.
double runtime_str_to_float(char* s) {
    if (s == NULL || *s == '\0') return 0.0;
    char* end;
    double val = strtod(s, &end);
    if (end == s || *end != '\0') return 0.0;
    return val;
}

// Case-sensitive equality.
int runtime_str_equals(char* a, char* b) {
    if (a == NULL) a = "";
    if (b == NULL) b = "";
    return strcmp(a, b) == 0;
}

// Lexicographic comparison. Returns -1, 0, or 1.
int runtime_str_compare(char* a, char* b) {
    if (a == NULL) a = "";
    if (b == NULL) b = "";
    int c = strcmp(a, b);
    if (c < 0) return -1;
    if (c > 0) return 1;
    return 0;
}

// -- Math --
double runtime_math_abs(double x) { return fabs(x); }
double runtime_math_sqrt(double x) { return sqrt(x); }
double runtime_math_min(double a, double b) { return a < b ? a : b; }
double runtime_math_max(double a, double b) { return a > b ? a : b; }
double runtime_math_clamp(double x, double lo, double hi) {
    double v = x < lo ? lo : x;
    return v > hi ? hi : v;
}
double runtime_math_pow(double a, double b) { return pow(a, b); }
double runtime_math_floor(double x) { return floor(x); }
double runtime_math_ceil(double x) { return ceil(x); }
double runtime_math_round(double x) { return round(x); }
double runtime_math_trunc(double x) { return trunc(x); }
double runtime_math_exp(double x) { return exp(x); }
double runtime_math_log(double x) { return log(x); }
double runtime_math_log10(double x) { return log10(x); }
double runtime_math_sin(double x) { return sin(x); }
double runtime_math_cos(double x) { return cos(x); }
double runtime_math_tan(double x) { return tan(x); }
double runtime_math_asin(double x) { return asin(x); }
double runtime_math_acos(double x) { return acos(x); }
double runtime_math_atan(double x) { return atan(x); }
double runtime_math_pi(void) { return M_PI; }
double runtime_math_e(void) { return M_E; }

// -- String helpers for Option/Result display --
// Tag values: Option{0=Some, 1=None}, Result{0=Success, 1=Error}
char* runtime_str_from_option(int64_t payload, int tag) {
    static char buf[64];
    if (tag == 1) {
        return "None";
    }
    snprintf(buf, sizeof(buf), "Some(%lld)", (long long)payload);
    return buf;
}

char* runtime_str_from_result(int64_t payload, int tag) {
    static char buf[64];
    if (tag == 0) {
        snprintf(buf, sizeof(buf), "Success(%lld)", (long long)payload);
    } else {
        snprintf(buf, sizeof(buf), "Error(%lld)", (long long)payload);
    }
    return buf;
}

// -- Time --
double runtime_time_now(void) {
#ifdef _WIN32
    FILETIME ft;
    GetSystemTimeAsFileTime(&ft);
    ULARGE_INTEGER u;
    u.LowPart = ft.dwLowDateTime;
    u.HighPart = ft.dwHighDateTime;
    // 100ns ticks since 1601-01-01; the Unix epoch offset is 11644473600s.
    return (double)(u.QuadPart / 10000000.0) - 11644473600.0;
#else
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec / 1e9;
#endif
}

int runtime_time_sleep(double secs) {
    if (secs > 0.0) {
#ifdef _WIN32
        Sleep((DWORD)(secs * 1000.0));
#else
        struct timespec ts;
        ts.tv_sec = (time_t)secs;
        ts.tv_nsec = (long)((secs - (double)ts.tv_sec) * 1e9);
        nanosleep(&ts, NULL);
#endif
    }
    return 1;
}

// -- stdio --
// Print an optional prompt, read one line from stdin and return it with the
// trailing newline stripped. Mirrors the interpreter `input` builtin.
char* runtime_input(char* prompt) {
    if (prompt != NULL) {
        fputs(prompt, stdout);
        fflush(stdout);
    }
    size_t cap = 128;
    size_t len = 0;
    char* buf = (char*)malloc(cap);
    if (buf == NULL) {
        runtime_panic("runtime_input: out of memory");
    }
    int c;
    while ((c = fgetc(stdin)) != EOF && c != '\n') {
        if (len + 1 >= cap) {
            cap *= 2;
            char* nb = (char*)realloc(buf, cap);
            if (nb == NULL) {
                free(buf);
                runtime_panic("runtime_input: out of memory");
            }
            buf = nb;
        }
        buf[len++] = (char)c;
    }
    while (len > 0 && (buf[len - 1] == '\n' || buf[len - 1] == '\r')) len--;
    buf[len] = '\0';
    return buf;
}

void runtime_eprint(char* s) {
    if (s == NULL) return;
    fputs(s, stderr);
}

void runtime_eprintln(char* s) {
    if (s == NULL) return;
    fputs(s, stderr);
    fputc('\n', stderr);
}

char* runtime_read_line(void) {
    size_t cap = 128;
    size_t len = 0;
    char* buf = (char*)malloc(cap);
    if (buf == NULL) {
        runtime_panic("runtime_read_line: out of memory");
    }
    int c;
    while ((c = fgetc(stdin)) != EOF && c != '\n') {
        if (len + 1 >= cap) {
            cap *= 2;
            char* nb = (char*)realloc(buf, cap);
            if (nb == NULL) {
                free(buf);
                runtime_panic("runtime_read_line: out of memory");
            }
            buf = nb;
        }
        buf[len++] = (char)c;
    }
    while (len > 0 && (buf[len - 1] == '\n' || buf[len - 1] == '\r')) len--;
    buf[len] = '\0';
    return buf;
}

char* runtime_read_all(void) {
    size_t cap = 4096;
    size_t len = 0;
    char* buf = (char*)malloc(cap);
    if (buf == NULL) {
        runtime_panic("runtime_read_all: out of memory");
    }
    int c;
    while ((c = fgetc(stdin)) != EOF) {
        if (len + 1 >= cap) {
            cap *= 2;
            char* nb = (char*)realloc(buf, cap);
            if (nb == NULL) {
                free(buf);
                runtime_panic("runtime_read_all: out of memory");
            }
            buf = nb;
        }
        buf[len++] = (char)c;
    }
    buf[len] = '\0';
    return buf;
}

int runtime_write_stdout(char* s) {
    if (s == NULL) return 0;
    size_t n = strlen(s);
    return (int)(fwrite(s, 1, n, stdout) == n);
}

int runtime_write_stderr(char* s) {
    if (s == NULL) return 0;
    size_t n = strlen(s);
    return (int)(fwrite(s, 1, n, stderr) == n);
}

// -- Filesystem --
char* runtime_read_file(char* path) {
    if (path == NULL) return NULL;
    FILE* f = fopen(path, "rb");
    if (f == NULL) return NULL;
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (size < 0) {
        fclose(f);
        return NULL;
    }
    char* r = (char*)malloc((size_t)size + 1);
    if (r == NULL) {
        fclose(f);
        runtime_panic("runtime_read_file: out of memory");
    }
    size_t got = fread(r, 1, (size_t)size, f);
    fclose(f);
    r[got] = '\0';
    return r;
}

int runtime_write_file(char* path, char* content) {
    if (path == NULL) return 0;
    if (content == NULL) content = "";
    FILE* f = fopen(path, "wb");
    if (f == NULL) return 0;
    size_t n = strlen(content);
    int ok = fwrite(content, 1, n, f) == n;
    fclose(f);
    return ok;
}

int runtime_append_file(char* path, char* content) {
    if (path == NULL) return 0;
    if (content == NULL) content = "";
    FILE* f = fopen(path, "ab");
    if (f == NULL) return 0;
    size_t n = strlen(content);
    int ok = fwrite(content, 1, n, f) == n;
    fclose(f);
    return ok;
}

int runtime_file_exists(char* path) {
    if (path == NULL) return 0;
#ifdef _WIN32
    return _access(path, 0) == 0;
#else
    return access(path, F_OK) == 0;
#endif
}

int runtime_remove_file(char* path) {
    if (path == NULL) return 0;
    return remove(path) == 0;
}

// Create a directory and any missing parents. Returns 1 on success (including
// when the path already exists as a directory).
int runtime_fs_create_dir(char* path) {
    if (path == NULL || *path == '\0') return 0;
    char* copy = (char*)malloc(strlen(path) + 1);
    if (copy == NULL) {
        runtime_panic("runtime_fs_create_dir: out of memory");
    }
    strcpy(copy, path);
    int ok = 1;
    for (char* p = copy + 1; *p; p++) {
        if (*p == '/' || *p == '\\') {
            char saved = *p;
            *p = '\0';
            if (copy[0]) {
#ifdef _WIN32
                if (_mkdir(copy) != 0 && _access(copy, 0) != 0) ok = 0;
#else
                if (mkdir(copy, 0777) != 0 && access(copy, F_OK) != 0) ok = 0;
#endif
            }
            *p = saved;
        }
    }
    if (copy[0]) {
#ifdef _WIN32
        if (_mkdir(copy) != 0 && _access(copy, 0) != 0) ok = 0;
#else
        if (mkdir(copy, 0777) != 0 && access(copy, F_OK) != 0) ok = 0;
#endif
    }
    free(copy);
    return ok;
}

int64_t runtime_fs_size(char* path) {
    if (path == NULL) return -1;
#ifdef _WIN32
    struct _stat st;
    if (_stat(path, &st) != 0) return -1;
    return (int64_t)st.st_size;
#else
    struct stat st;
    if (stat(path, &st) != 0) return -1;
    return (int64_t)st.st_size;
#endif
}

void runtime_fs_metadata(char* path, int64_t* size, int8_t* is_dir, int8_t* is_file) {
    if (size != NULL) *size = -1;
    if (is_dir != NULL) *is_dir = 0;
    if (is_file != NULL) *is_file = 0;
    if (path == NULL) return;
#ifdef _WIN32
    struct _stat st;
    if (_stat(path, &st) != 0) return;
    if (size != NULL) *size = (int64_t)st.st_size;
    if (is_dir != NULL) *is_dir = (st.st_mode & _S_IFDIR) != 0;
    if (is_file != NULL) *is_file = (st.st_mode & _S_IFREG) != 0;
#else
    struct stat st;
    if (stat(path, &st) != 0) return;
    if (size != NULL) *size = (int64_t)st.st_size;
    if (is_dir != NULL) *is_dir = S_ISDIR(st.st_mode);
    if (is_file != NULL) *is_file = S_ISREG(st.st_mode);
#endif
}

// List the immediate children of `path` as full paths (interpreter
// `fs_list_dir`). Each entry is a malloc'd string stored as an i64 slot.
LimeList runtime_fs_list_dir(char* path) {
    LimeList list = runtime_list_empty();
    if (path == NULL) return list;
#ifdef _WIN32
    size_t plen = strlen(path);
    char* pattern = (char*)malloc(plen + 3);
    if (pattern == NULL) {
        runtime_panic("runtime_fs_list_dir: out of memory");
    }
    memcpy(pattern, path, plen);
    if (plen > 0 && (path[plen - 1] == '\\' || path[plen - 1] == '/')) {
        pattern[plen] = '*';
        pattern[plen + 1] = '\0';
    } else {
        pattern[plen] = '\\';
        pattern[plen + 1] = '*';
        pattern[plen + 2] = '\0';
    }
    struct _finddata_t fd;
    intptr_t handle = _findfirst(pattern, &fd);
    free(pattern);
    if (handle == (intptr_t)-1) return list;
    do {
        if (strcmp(fd.name, ".") == 0 || strcmp(fd.name, "..") == 0) continue;
        size_t nl = strlen(fd.name);
        char* full = (char*)malloc(plen + 2 + nl);
        if (full == NULL) {
            runtime_panic("runtime_fs_list_dir: out of memory");
        }
        memcpy(full, path, plen);
        if (plen > 0 && (path[plen - 1] == '\\' || path[plen - 1] == '/')) {
            memcpy(full + plen, fd.name, nl);
            full[plen + nl] = '\0';
        } else {
            full[plen] = '\\';
            memcpy(full + plen + 1, fd.name, nl);
            full[plen + 1 + nl] = '\0';
        }
        list = runtime_list_add(list, (int64_t)(intptr_t)full);
    } while (_findnext(handle, &fd) == 0);
    _findclose(handle);
    return list;
#else
    DIR* d = opendir(path);
    if (d == NULL) return list;
    struct dirent* e;
    size_t plen = strlen(path);
    while ((e = readdir(d)) != NULL) {
        if (strcmp(e->d_name, ".") == 0 || strcmp(e->d_name, "..") == 0) continue;
        size_t nl = strlen(e->d_name);
        char* full = (char*)malloc(plen + 2 + nl);
        if (full == NULL) {
            runtime_panic("runtime_fs_list_dir: out of memory");
        }
        memcpy(full, path, plen);
        full[plen] = '/';
        memcpy(full + plen + 1, e->d_name, nl);
        full[plen + 1 + nl] = '\0';
        list = runtime_list_add(list, (int64_t)(intptr_t)full);
    }
    closedir(d);
    return list;
#endif
}

int runtime_fs_copy(char* src, char* dst) {
    if (src == NULL || dst == NULL) return 0;
    FILE* fin = fopen(src, "rb");
    if (fin == NULL) return 0;
    FILE* fout = fopen(dst, "wb");
    if (fout == NULL) { fclose(fin); return 0; }
    char buf[8192];
    size_t n;
    int ok = 1;
    while ((n = fread(buf, 1, sizeof(buf), fin)) > 0) {
        if (fwrite(buf, 1, n, fout) != n) { ok = 0; break; }
    }
    fclose(fin);
    fclose(fout);
    return ok;
}

int runtime_fs_rename(char* src, char* dst) {
    if (src == NULL || dst == NULL) return 0;
    return rename(src, dst) == 0;
}

int runtime_fs_is_file(char* path) {
    if (path == NULL) return 0;
#ifdef _WIN32
    struct _stat st;
    if (_stat(path, &st) != 0) return 0;
    return (st.st_mode & _S_IFREG) != 0;
#else
    struct stat st;
    if (stat(path, &st) != 0) return 0;
    return S_ISREG(st.st_mode);
#endif
}

int runtime_fs_is_dir(char* path) {
    if (path == NULL) return 0;
#ifdef _WIN32
    struct _stat st;
    if (_stat(path, &st) != 0) return 0;
    return (st.st_mode & _S_IFDIR) != 0;
#else
    struct stat st;
    if (stat(path, &st) != 0) return 0;
    return S_ISDIR(st.st_mode);
#endif
}

int runtime_fs_remove_dir(char* path) {
    if (path == NULL) return 0;
    return rmdir(path) == 0;
}

LimeList runtime_fs_read_lines(char* path) {
    LimeList list = runtime_list_empty();
    if (path == NULL) return list;
    char* content = runtime_read_file(path);
    if (content == NULL) return list;
    char* line = content;
    while (*line) {
        char* end = strchr(line, '\n');
        size_t len;
        if (end) {
            len = (size_t)(end - line);
            /* strip trailing \r */
            if (len > 0 && line[len - 1] == '\r') len--;
            end++;
        } else {
            len = strlen(line);
            if (len > 0 && line[len - 1] == '\r') len--;
        }
        char* s = (char*)malloc(len + 1);
        if (s == NULL) {
            runtime_panic("runtime_fs_read_lines: out of memory");
        }
        memcpy(s, line, len);
        s[len] = '\0';
        list = runtime_list_add(list, (int64_t)(intptr_t)s);
        if (end) line = end; else break;
    }
    free(content);
    return list;
}

int runtime_fs_write_lines(char* path, LimeList lines) {
    if (path == NULL) return 0;
    FILE* f = fopen(path, "wb");
    if (f == NULL) return 0;
    int ok = 1;
    for (int64_t i = 0; i < lines.len; i++) {
        char* s = (char*)(intptr_t)lines.data[i];
        if (s == NULL) s = "";
        size_t n = strlen(s);
        if (fwrite(s, 1, n, f) != n) { ok = 0; break; }
        if (fwrite("\n", 1, 1, f) != 1) { ok = 0; break; }
    }
    fclose(f);
    return ok;
}

// -- List helpers --

// Grow a LimeList buffer so capacity doubles (starts at 4) until it exceeds
// the current length. Aborts with a panic message on out-of-memory.
static void grow_list(LimeList* list) {
    int64_t new_cap = list->cap == 0 ? 4 : list->cap * 2;
    while (new_cap <= list->len) {
        new_cap *= 2;
    }
    char* new_data = (char*)realloc(list->data, (size_t)(new_cap * 8));
    if (new_data == NULL) {
        runtime_panic("runtime_list: out of memory");
    }
    list->data = new_data;
    list->cap = new_cap;
}

// UTF-8 decode one sequence starting at s[offset]; returns the code point and
// the number of bytes consumed. Invalid leading bytes are stored raw.
static int64_t decode_utf8(const char* s, int64_t offset, int64_t* consumed) {
    unsigned char b = (unsigned char)s[offset];
    if (b < 0x80) {
        *consumed = 1;
        return (int64_t)b;
    } else if ((b & 0xE0) == 0xC0) {
        *consumed = 2;
        return ((int64_t)(b & 0x1F) << 6)
             | ((unsigned char)s[offset + 1] & 0x3F);
    } else if ((b & 0xF0) == 0xE0) {
        *consumed = 3;
        return ((int64_t)(b & 0x0F) << 12)
             | ((int64_t)((unsigned char)s[offset + 1] & 0x3F) << 6)
             | ((unsigned char)s[offset + 2] & 0x3F);
    } else if ((b & 0xF8) == 0xF0) {
        *consumed = 4;
        return ((int64_t)(b & 0x07) << 18)
             | ((int64_t)((unsigned char)s[offset + 1] & 0x3F) << 12)
             | ((int64_t)((unsigned char)s[offset + 2] & 0x3F) << 6)
             | ((unsigned char)s[offset + 3] & 0x3F);
    }
    *consumed = 1;
    return (int64_t)b;
}

// Decode the string as UTF-8 and return a list of code points.
LimeList runtime_str_chars(char* s) {
    LimeList list;
    list.data = NULL;
    list.len = 0;
    list.cap = 0;
    if (s == NULL) return list;

    int64_t offset = 0;
    while (s[offset] != '\0') {
        int64_t consumed;
        int64_t cp = decode_utf8(s, offset, &consumed);
        if (list.len >= list.cap) grow_list(&list);
        ((int64_t*)list.data)[list.len] = cp;
        list.len++;
        offset += consumed;
    }
    return list;
}

// Return a list of the raw byte values of the string.
LimeList runtime_str_bytes(char* s) {
    LimeList list;
    list.data = NULL;
    list.len = 0;
    list.cap = 0;
    if (s == NULL) return list;

    int64_t offset = 0;
    while (s[offset] != '\0') {
        if (list.len >= list.cap) grow_list(&list);
        ((int64_t*)list.data)[list.len] = (int64_t)(unsigned char)s[offset];
        list.len++;
        offset++;
    }
    return list;
}

// -- List operations --

// Construct an empty list.
LimeList runtime_list_empty(void) {
    LimeList list;
    list.data = NULL;
    list.len = 0;
    list.cap = 0;
    return list;
}

// Append an element, growing the buffer as needed.
LimeList runtime_list_add(LimeList list, int64_t elem) {
    if (list.len >= list.cap) grow_list(&list);
    ((int64_t*)list.data)[list.len] = elem;
    list.len++;
    return list;
}

// Replace the element at `index` (no-op when the index is out of bounds).
LimeList runtime_list_set(LimeList list, int64_t index, int64_t elem) {
    if (index >= 0 && index < list.len) {
        ((int64_t*)list.data)[index] = elem;
    }
    return list;
}

// Return the number of elements in the list.
int64_t runtime_list_len(LimeList list) {
    return list.len;
}

// Return the element at `index`. Returns 0 if out of bounds.
int64_t runtime_list_get(LimeList list, int64_t index) {
    if (index >= 0 && index < list.len) {
        return ((int64_t*)list.data)[index];
    }
    return 0;
}

// -- List mutation / inspection (Phase C-1.2) --

// Insert `elem` at `index`, shifting elements right. Clamps index to [0, len].
LimeList runtime_list_insert(LimeList list, int64_t index, int64_t elem) {
    if (index < 0) index = 0;
    if (index > list.len) index = list.len;
    if (list.len >= list.cap) grow_list(&list);
    int64_t* arr = (int64_t*)list.data;
    memmove(arr + index + 1, arr + index, (list.len - index) * sizeof(int64_t));
    arr[index] = elem;
    list.len++;
    return list;
}

LimeList runtime_list_clear(LimeList list) {
    list.len = 0;
    return list;
}

// Sort the list elements (simple insertion sort for i64 values).
LimeList runtime_list_sort(LimeList list) {
    int64_t* arr = (int64_t*)list.data;
    for (int64_t i = 1; i < list.len; i++) {
        int64_t key = arr[i];
        int64_t j = i - 1;
        while (j >= 0 && arr[j] > key) {
            arr[j + 1] = arr[j];
            j--;
        }
        arr[j + 1] = key;
    }
    return list;
}

LimeList runtime_list_clone(LimeList list) {
    LimeList result = {0, 0, 0};
    if (list.len > 0) {
        if (result.len >= result.cap) grow_list(&result);
        while (result.cap < list.len) grow_list(&result);
        memcpy(result.data, list.data, list.len * sizeof(int64_t));
        result.len = list.len;
    }
    return result;
}

// -- Map operations --

static LimeMap map_grow(LimeMap map, int64_t min_cap) {
    if (map.cap >= min_cap) return map;
    int64_t new_cap = map.cap > 0 ? map.cap * 2 : 4;
    while (new_cap < min_cap) new_cap *= 2;
    void* new_data = realloc(map.data, new_cap * 2 * sizeof(int64_t));
    if (!new_data) runtime_panic("map: out of memory");
    map.data = new_data;
    map.cap = new_cap;
    return map;
}

int64_t runtime_map_len(LimeMap map) {
    return map.len;
}

int runtime_map_is_empty(LimeMap map) {
    return map.len == 0 ? 1 : 0;
}

LimeMap runtime_map_insert(LimeMap map, int64_t key, int64_t val) {
    for (int64_t i = 0; i < map.len; i++) {
        int64_t* pair = (int64_t*)map.data + i * 2;
        if (pair[0] == key) {
            pair[1] = val;
            return map;
        }
    }
    if (map.len >= map.cap) map = map_grow(map, map.len + 1);
    int64_t* pair = (int64_t*)map.data + map.len * 2;
    pair[0] = key;
    pair[1] = val;
    map.len++;
    return map;
}

int64_t runtime_map_get(LimeMap map, int64_t key) {
    for (int64_t i = 0; i < map.len; i++) {
        int64_t* pair = (int64_t*)map.data + i * 2;
        if (pair[0] == key) return pair[1];
    }
    return 0;
}

LimeMap runtime_map_remove(LimeMap map, int64_t key) {
    for (int64_t i = 0; i < map.len; i++) {
        int64_t* pair = (int64_t*)map.data + i * 2;
        if (pair[0] == key) {
            int64_t* dst = (int64_t*)map.data + i * 2;
            int64_t* src = (int64_t*)map.data + (i + 1) * 2;
            int64_t remaining = map.len - i - 1;
            if (remaining > 0) {
                memmove(dst, src, remaining * 2 * sizeof(int64_t));
            }
            map.len--;
            return map;
        }
    }
    return map;
}

int runtime_map_contains_key(LimeMap map, int64_t key) {
    for (int64_t i = 0; i < map.len; i++) {
        int64_t* pair = (int64_t*)map.data + i * 2;
        if (pair[0] == key) return 1;
    }
    return 0;
}

LimeMap runtime_map_clear(LimeMap map) {
    map.len = 0;
    return map;
}

LimeMap runtime_map_clone(LimeMap map) {
    LimeMap result = {0, 0, 0};
    if (map.len > 0) {
        result = map_grow(result, map.len);
        memcpy(result.data, map.data, map.len * 2 * sizeof(int64_t));
        result.len = map.len;
    }
    return result;
}

// -- Set operations --

static LimeSet set_grow(LimeSet set, int64_t min_cap) {
    if (set.cap >= min_cap) return set;
    int64_t new_cap = set.cap > 0 ? set.cap * 2 : 4;
    while (new_cap < min_cap) new_cap *= 2;
    void* new_data = realloc(set.data, new_cap * sizeof(int64_t));
    if (!new_data) runtime_panic("set: out of memory");
    set.data = new_data;
    set.cap = new_cap;
    return set;
}

int64_t runtime_set_len(LimeSet set) {
    return set.len;
}

int runtime_set_is_empty(LimeSet set) {
    return set.len == 0 ? 1 : 0;
}

LimeSet runtime_set_add(LimeSet set, int64_t elem) {
    for (int64_t i = 0; i < set.len; i++) {
        if (((int64_t*)set.data)[i] == elem) return set;
    }
    if (set.len >= set.cap) set = set_grow(set, set.len + 1);
    ((int64_t*)set.data)[set.len] = elem;
    set.len++;
    return set;
}

LimeSet runtime_set_remove(LimeSet set, int64_t elem) {
    for (int64_t i = 0; i < set.len; i++) {
        if (((int64_t*)set.data)[i] == elem) {
            memmove((int64_t*)set.data + i, (int64_t*)set.data + i + 1, (set.len - i - 1) * sizeof(int64_t));
            set.len--;
            return set;
        }
    }
    return set;
}

int runtime_set_contains(LimeSet set, int64_t elem) {
    for (int64_t i = 0; i < set.len; i++) {
        if (((int64_t*)set.data)[i] == elem) return 1;
    }
    return 0;
}

LimeSet runtime_set_clear(LimeSet set) {
    set.len = 0;
    return set;
}

LimeSet runtime_set_clone(LimeSet set) {
    LimeSet result = {0, 0, 0};
    if (set.len > 0) {
        result = set_grow(result, set.len);
        memcpy(result.data, set.data, set.len * sizeof(int64_t));
        result.len = set.len;
    }
    return result;
}

// -- Queue operations (FIFO: push at back, pop from front) --

LimeList runtime_queue_push(LimeList queue, int64_t elem) {
    return runtime_list_add(queue, elem);
}

int64_t runtime_queue_pop(LimeList queue) {
    if (queue.len == 0) runtime_panic("queue_pop: empty queue");
    int64_t val = ((int64_t*)queue.data)[0];
    memmove((int64_t*)queue.data, (int64_t*)queue.data + 1, (queue.len - 1) * sizeof(int64_t));
    queue.len--;
    return val;
}

int64_t runtime_queue_front(LimeList queue) {
    if (queue.len == 0) runtime_panic("queue_front: empty queue");
    return ((int64_t*)queue.data)[0];
}

int64_t runtime_queue_back(LimeList queue) {
    if (queue.len == 0) runtime_panic("queue_back: empty queue");
    return ((int64_t*)queue.data)[queue.len - 1];
}

int64_t runtime_queue_len(LimeList queue) {
    return queue.len;
}

int runtime_queue_is_empty(LimeList queue) {
    return queue.len == 0 ? 1 : 0;
}

LimeList runtime_queue_clear(LimeList queue) {
    queue.len = 0;
    return queue;
}

// -- Stack operations (LIFO: push at back, pop from back) --

LimeList runtime_stack_push(LimeList stack, int64_t elem) {
    return runtime_list_add(stack, elem);
}

int64_t runtime_stack_pop(LimeList stack) {
    if (stack.len == 0) runtime_panic("stack_pop: empty stack");
    stack.len--;
    return ((int64_t*)stack.data)[stack.len];
}

int64_t runtime_stack_peek(LimeList stack) {
    if (stack.len == 0) runtime_panic("stack_peek: empty stack");
    return ((int64_t*)stack.data)[stack.len - 1];
}

int64_t runtime_stack_len(LimeList stack) {
    return stack.len;
}

int runtime_stack_is_empty(LimeList stack) {
    return stack.len == 0 ? 1 : 0;
}

LimeList runtime_stack_clear(LimeList stack) {
    stack.len = 0;
    return stack;
}

// -- Closure / function values (Phase B-2.2) --

// Create a closure wrapping a function pointer and an environment pointer.
LimeClosure* runtime_make_closure(void* fn_ptr, void* env_ptr) {
    LimeClosure* c = (LimeClosure*)malloc(sizeof(LimeClosure));
    if (c == NULL) {
        runtime_panic("runtime_make_closure: out of memory");
    }
    c->fn_ptr = fn_ptr;
    c->env_ptr = env_ptr;
    return c;
}

// Call a closure's function with packed arguments, returning i64.
// The function signature is: int64_t fn(i8* env, i8* packed_args)
typedef int64_t (*ClosureFnI64)(void*, void*);
int64_t runtime_call_closure_i64(LimeClosure* closure, void* packed_args) {
    if (closure == NULL) {
        runtime_panic("runtime_call_closure_i64: null closure");
    }
    if (closure->fn_ptr == NULL) {
        runtime_panic("runtime_call_closure_i64: null function pointer");
    }
    ClosureFnI64 fn = (ClosureFnI64)closure->fn_ptr;
    return fn(closure->env_ptr, packed_args);
}

// Call a closure's function with packed arguments, returning i8* (ptr).
// The function signature is: i8* fn(i8* env, i8* packed_args)
typedef void* (*ClosureFnPtr)(void*, void*);
void* runtime_call_closure_ptr(LimeClosure* closure, void* packed_args) {
    if (closure == NULL) {
        runtime_panic("runtime_call_closure_ptr: null closure");
    }
    if (closure->fn_ptr == NULL) {
        runtime_panic("runtime_call_closure_ptr: null function pointer");
    }
    ClosureFnPtr fn = (ClosureFnPtr)closure->fn_ptr;
    return fn(closure->env_ptr, packed_args);
}

// Wrap a plain function pointer (no environment) into a closure.
LimeClosure* runtime_make_fn_ref(void* fn_ptr) {
    return runtime_make_closure(fn_ptr, NULL);
}

// ===== JSON runtime =====

static LimeJson* json_alloc(LimeJsonTag tag) {
    LimeJson* j = (LimeJson*)malloc(sizeof(LimeJson));
    if (!j) runtime_panic("json: out of memory");
    j->tag = tag;
    memset(&j->data, 0, sizeof(j->data));
    return j;
}

static void json_free(LimeJson* j) {
    if (!j) return;
    switch (j->tag) {
        case JSON_STRING: free(j->data.string_val); break;
        case JSON_ARRAY:
            for (int64_t i = 0; i < j->data.array_val.len; i++)
                json_free(j->data.array_val.items[i]);
            free(j->data.array_val.items);
            break;
        case JSON_OBJECT:
            for (int64_t i = 0; i < j->data.object_val.len; i++) {
                free(j->data.object_val.keys[i]);
                json_free(j->data.object_val.values[i]);
            }
            free(j->data.object_val.keys);
            free(j->data.object_val.values);
            break;
        default: break;
    }
    free(j);
}

static char* json_strdup(const char* s) {
    size_t len = strlen(s);
    char* d = (char*)malloc(len + 1);
    if (!d) runtime_panic("json: out of memory");
    memcpy(d, s, len + 1);
    return d;
}

// Forward declaration for recursive stringify
static void json_stringify_impl(LimeJson* j, char** buf, int64_t* len, int64_t* cap);

static void json_ensure(char** buf, int64_t* len, int64_t* cap, int64_t needed) {
    while (*len + needed >= *cap) {
        *cap = (*cap) * 2 + 256;
        *buf = (char*)realloc(*buf, *cap);
        if (!*buf) runtime_panic("json: out of memory");
    }
}

static void json_push_str(char** buf, int64_t* len, int64_t* cap, const char* s) {
    int64_t slen = (int64_t)strlen(s);
    json_ensure(buf, len, cap, slen);
    memcpy(*buf + *len, s, slen);
    *len += slen;
}

static void json_push_char(char** buf, int64_t* len, int64_t* cap, char c) {
    json_ensure(buf, len, cap, 1);
    (*buf)[(*len)++] = c;
}

static void json_stringify_string(const char* s, char** buf, int64_t* len, int64_t* cap) {
    json_push_char(buf, len, cap, '"');
    for (const char* p = s; *p; p++) {
        switch (*p) {
            case '"':  json_push_str(buf, len, cap, "\\\""); break;
            case '\\': json_push_str(buf, len, cap, "\\\\"); break;
            case '\b': json_push_str(buf, len, cap, "\\b"); break;
            case '\f': json_push_str(buf, len, cap, "\\f"); break;
            case '\n': json_push_str(buf, len, cap, "\\n"); break;
            case '\r': json_push_str(buf, len, cap, "\\r"); break;
            case '\t': json_push_str(buf, len, cap, "\\t"); break;
            default: json_push_char(buf, len, cap, *p); break;
        }
    }
    json_push_char(buf, len, cap, '"');
}

static void json_stringify_impl(LimeJson* j, char** buf, int64_t* len, int64_t* cap) {
    if (!j) { json_push_str(buf, len, cap, "null"); return; }
    char num[64];
    switch (j->tag) {
        case JSON_NULL:   json_push_str(buf, len, cap, "null"); break;
        case JSON_BOOL:   json_push_str(buf, len, cap, j->data.bool_val ? "true" : "false"); break;
        case JSON_INT:
            snprintf(num, sizeof(num), "%lld", (long long)j->data.int_val);
            json_push_str(buf, len, cap, num);
            break;
        case JSON_FLOAT:
            snprintf(num, sizeof(num), "%.17g", j->data.float_val);
            json_push_str(buf, len, cap, num);
            break;
        case JSON_STRING:
            json_stringify_string(j->data.string_val, buf, len, cap);
            break;
        case JSON_ARRAY:
            json_push_char(buf, len, cap, '[');
            for (int64_t i = 0; i < j->data.array_val.len; i++) {
                if (i > 0) json_push_char(buf, len, cap, ',');
                json_stringify_impl(j->data.array_val.items[i], buf, len, cap);
            }
            json_push_char(buf, len, cap, ']');
            break;
        case JSON_OBJECT:
            json_push_char(buf, len, cap, '{');
            for (int64_t i = 0; i < j->data.object_val.len; i++) {
                if (i > 0) json_push_char(buf, len, cap, ',');
                json_stringify_string(j->data.object_val.keys[i], buf, len, cap);
                json_push_char(buf, len, cap, ':');
                json_stringify_impl(j->data.object_val.values[i], buf, len, cap);
            }
            json_push_char(buf, len, cap, '}');
            break;
    }
}

char* runtime_json_stringify(LimeJson* j) {
    char* buf = (char*)malloc(256);
    int64_t len = 0, cap = 256;
    if (!buf) runtime_panic("json: out of memory");
    json_stringify_impl(j, &buf, &len, &cap);
    buf[len] = '\0';
    return buf;
}

// JSON parser
static const char* jp_input;
static int64_t jp_pos;

static void jp_skip_ws(void) {
    while (jp_input[jp_pos] == ' ' || jp_input[jp_pos] == '\t' ||
           jp_input[jp_pos] == '\n' || jp_input[jp_pos] == '\r')
        jp_pos++;
}

static LimeJson* jp_parse_value(void);

static LimeJson* jp_parse_string_raw(void) {
    jp_pos++; // skip '"'
    char buf[4096];
    int64_t blen = 0;
    while (jp_input[jp_pos] != '"' && jp_input[jp_pos] != '\0') {
        if (jp_input[jp_pos] == '\\') {
            jp_pos++;
            char esc = jp_input[jp_pos];
            switch (esc) {
                case '"': case '\\': case '/': buf[blen++] = esc; break;
                case 'b': buf[blen++] = '\b'; break;
                case 'f': buf[blen++] = '\f'; break;
                case 'n': buf[blen++] = '\n'; break;
                case 'r': buf[blen++] = '\r'; break;
                case 't': buf[blen++] = '\t'; break;
                case 'u': {
                    jp_pos++;
                    char hex[5] = {0};
                    for (int i = 0; i < 4 && jp_input[jp_pos]; i++, jp_pos++)
                        hex[i] = jp_input[jp_pos];
                    jp_pos--; // loop will increment
                    unsigned long code = strtoul(hex, NULL, 16);
                    if (code < 0x80) { buf[blen++] = (char)code; }
                    else if (code < 0x800) { buf[blen++] = (char)(0xC0|(code>>6)); buf[blen++] = (char)(0x80|(code&0x3F)); }
                    else { buf[blen++] = (char)(0xE0|(code>>12)); buf[blen++] = (char)(0x80|((code>>6)&0x3F)); buf[blen++] = (char)(0x80|(code&0x3F)); }
                    break;
                }
                default: buf[blen++] = esc; break;
            }
        } else {
            buf[blen++] = jp_input[jp_pos];
        }
        jp_pos++;
    }
    jp_pos++; // skip closing '"'
    buf[blen] = '\0';
    LimeJson* j = json_alloc(JSON_STRING);
    j->data.string_val = json_strdup(buf);
    return j;
}

static LimeJson* jp_parse_number(void) {
    int64_t start = jp_pos;
    int is_float = 0;
    if (jp_input[jp_pos] == '-') jp_pos++;
    while (jp_input[jp_pos] >= '0' && jp_input[jp_pos] <= '9') jp_pos++;
    if (jp_input[jp_pos] == '.') { is_float = 1; jp_pos++; while (jp_input[jp_pos] >= '0' && jp_input[jp_pos] <= '9') jp_pos++; }
    if (jp_input[jp_pos] == 'e' || jp_input[jp_pos] == 'E') { is_float = 1; jp_pos++; if (jp_input[jp_pos]=='+'||jp_input[jp_pos]=='-') jp_pos++; while (jp_input[jp_pos] >= '0' && jp_input[jp_pos] <= '9') jp_pos++; }
    int64_t slen = jp_pos - start;
    char* s = (char*)malloc(slen + 1);
    memcpy(s, jp_input + start, slen);
    s[slen] = '\0';
    LimeJson* j;
    if (is_float) {
        j = json_alloc(JSON_FLOAT);
        j->data.float_val = atof(s);
    } else {
        j = json_alloc(JSON_INT);
        j->data.int_val = strtoll(s, NULL, 10);
    }
    free(s);
    return j;
}

static LimeJson* jp_parse_object(void) {
    jp_pos++; // skip '{'
    jp_skip_ws();
    LimeJson* j = json_alloc(JSON_OBJECT);
    int64_t cap = 8;
    j->data.object_val.keys = (char**)malloc(cap * sizeof(char*));
    j->data.object_val.values = (LimeJson**)malloc(cap * sizeof(LimeJson*));
    j->data.object_val.len = 0;
    j->data.object_val.cap = cap;
    jp_skip_ws();
    if (jp_input[jp_pos] == '}') { jp_pos++; return j; }
    for (;;) {
        jp_skip_ws();
        LimeJson* key = jp_parse_string_raw();
        char* key_str = key->data.string_val;
        key->data.string_val = NULL;
        json_free(key);
        jp_skip_ws();
        jp_pos++; // skip ':'
        LimeJson* val = jp_parse_value();
        if (j->data.object_val.len >= j->data.object_val.cap) {
            j->data.object_val.cap *= 2;
            j->data.object_val.keys = (char**)realloc(j->data.object_val.keys, j->data.object_val.cap * sizeof(char*));
            j->data.object_val.values = (LimeJson**)realloc(j->data.object_val.values, j->data.object_val.cap * sizeof(LimeJson*));
        }
        j->data.object_val.keys[j->data.object_val.len] = key_str;
        j->data.object_val.values[j->data.object_val.len] = val;
        j->data.object_val.len++;
        jp_skip_ws();
        if (jp_input[jp_pos] == '}') { jp_pos++; break; }
        jp_pos++; // skip ','
    }
    return j;
}

static LimeJson* jp_parse_array(void) {
    jp_pos++; // skip '['
    jp_skip_ws();
    LimeJson* j = json_alloc(JSON_ARRAY);
    int64_t cap = 8;
    j->data.array_val.items = (LimeJson**)malloc(cap * sizeof(LimeJson*));
    j->data.array_val.len = 0;
    j->data.array_val.cap = cap;
    if (jp_input[jp_pos] == ']') { jp_pos++; return j; }
    for (;;) {
        LimeJson* val = jp_parse_value();
        if (j->data.array_val.len >= j->data.array_val.cap) {
            j->data.array_val.cap *= 2;
            j->data.array_val.items = (LimeJson**)realloc(j->data.array_val.items, j->data.array_val.cap * sizeof(LimeJson*));
        }
        j->data.array_val.items[j->data.array_val.len++] = val;
        jp_skip_ws();
        if (jp_input[jp_pos] == ']') { jp_pos++; break; }
        jp_pos++; // skip ','
    }
    return j;
}

static LimeJson* jp_parse_value(void) {
    jp_skip_ws();
    char c = jp_input[jp_pos];
    if (c == '{') return jp_parse_object();
    if (c == '[') return jp_parse_array();
    if (c == '"') return jp_parse_string_raw();
    if (c == 't' || c == 'f') {
        LimeJson* j = json_alloc(JSON_BOOL);
        j->data.bool_val = (c == 't') ? 1 : 0;
        if (c == 't') jp_pos += 4; else jp_pos += 5;
        return j;
    }
    if (c == 'n') {
        jp_pos += 4;
        return json_alloc(JSON_NULL);
    }
    if (c == '-' || (c >= '0' && c <= '9')) return jp_parse_number();
    runtime_panic("json: unexpected character");
    return NULL;
}

LimeJson* runtime_json_parse(char* s) {
    jp_input = s;
    jp_pos = 0;
    LimeJson* result = jp_parse_value();
    return result;
}

LimeJson* runtime_json_get(LimeJson* j, char* key) {
    if (!j || j->tag != JSON_OBJECT) return NULL;
    for (int64_t i = 0; i < j->data.object_val.len; i++) {
        if (strcmp(j->data.object_val.keys[i], key) == 0)
            return j->data.object_val.values[i];
    }
    return NULL;
}

int8_t runtime_json_has(LimeJson* j, char* key) {
    if (!j || j->tag != JSON_OBJECT) return 0;
    for (int64_t i = 0; i < j->data.object_val.len; i++) {
        if (strcmp(j->data.object_val.keys[i], key) == 0) return 1;
    }
    return 0;
}

int64_t runtime_json_len(LimeJson* j) {
    if (!j) return 0;
    switch (j->tag) {
        case JSON_ARRAY:  return j->data.array_val.len;
        case JSON_OBJECT: return j->data.object_val.len;
        case JSON_STRING: return (int64_t)strlen(j->data.string_val);
        default: return 0;
    }
}

LimeJson* runtime_json_at(LimeJson* j, int64_t index) {
    if (!j || j->tag != JSON_ARRAY) return NULL;
    if (index < 0 || index >= j->data.array_val.len) return NULL;
    return j->data.array_val.items[index];
}

char* runtime_json_as_string(LimeJson* j) {
    if (!j) return json_strdup("");
    if (j->tag == JSON_STRING) return json_strdup(j->data.string_val);
    char* result = runtime_json_stringify(j);
    return result;
}

int64_t runtime_json_as_int(LimeJson* j) {
    if (!j) return 0;
    switch (j->tag) {
        case JSON_INT:   return j->data.int_val;
        case JSON_FLOAT: return (int64_t)j->data.float_val;
        case JSON_BOOL:  return j->data.bool_val ? 1 : 0;
        default: return 0;
    }
}

double runtime_json_as_float(LimeJson* j) {
    if (!j) return 0.0;
    switch (j->tag) {
        case JSON_FLOAT: return j->data.float_val;
        case JSON_INT:   return (double)j->data.int_val;
        default: return 0.0;
    }
}

int8_t runtime_json_as_bool(LimeJson* j) {
    if (!j) return 0;
    switch (j->tag) {
        case JSON_BOOL:  return j->data.bool_val;
        case JSON_INT:   return j->data.int_val != 0 ? 1 : 0;
        case JSON_FLOAT: return j->data.float_val != 0.0 ? 1 : 0;
        case JSON_STRING: return j->data.string_val[0] != '\0' ? 1 : 0;
        case JSON_ARRAY:  return j->data.array_val.len > 0 ? 1 : 0;
        case JSON_OBJECT: return j->data.object_val.len > 0 ? 1 : 0;
        default: return 0;
    }
}

LimeJson* runtime_json_null(void) {
    return json_alloc(JSON_NULL);
}

LimeJson* runtime_json_object(void) {
    LimeJson* j = json_alloc(JSON_OBJECT);
    j->data.object_val.keys = NULL;
    j->data.object_val.values = NULL;
    j->data.object_val.len = 0;
    j->data.object_val.cap = 0;
    return j;
}

LimeJson* runtime_json_array(void) {
    LimeJson* j = json_alloc(JSON_ARRAY);
    j->data.array_val.items = NULL;
    j->data.array_val.len = 0;
    j->data.array_val.cap = 0;
    return j;
}

int8_t runtime_json_set(LimeJson* j, char* key, LimeJson* val) {
    if (!j || j->tag != JSON_OBJECT) return 0;
    // Remove existing key
    for (int64_t i = 0; i < j->data.object_val.len; i++) {
        if (strcmp(j->data.object_val.keys[i], key) == 0) {
            free(j->data.object_val.keys[i]);
            json_free(j->data.object_val.values[i]);
            j->data.object_val.keys[i] = json_strdup(key);
            j->data.object_val.values[i] = val;
            return 1;
        }
    }
    // Add new key
    if (j->data.object_val.len >= j->data.object_val.cap) {
        int64_t new_cap = j->data.object_val.cap ? j->data.object_val.cap * 2 : 8;
        j->data.object_val.keys = (char**)realloc(j->data.object_val.keys, new_cap * sizeof(char*));
        j->data.object_val.values = (LimeJson**)realloc(j->data.object_val.values, new_cap * sizeof(LimeJson*));
        j->data.object_val.cap = new_cap;
    }
    j->data.object_val.keys[j->data.object_val.len] = json_strdup(key);
    j->data.object_val.values[j->data.object_val.len] = val;
    j->data.object_val.len++;
    return 1;
}

int8_t runtime_json_push(LimeJson* j, LimeJson* elem) {
    if (!j || j->tag != JSON_ARRAY) return 0;
    if (j->data.array_val.len >= j->data.array_val.cap) {
        int64_t new_cap = j->data.array_val.cap ? j->data.array_val.cap * 2 : 8;
        j->data.array_val.items = (LimeJson**)realloc(j->data.array_val.items, new_cap * sizeof(LimeJson*));
        j->data.array_val.cap = new_cap;
    }
    j->data.array_val.items[j->data.array_val.len++] = elem;
    return 1;
}

// ========================================================================
// Path operations (Phase C-1.8)
// ========================================================================

static int path_is_sep(char c) {
    return c == '/' || c == '\\';
}

// Skip trailing separators (but keep root).
static char* path_skip_trailing_seps(char* s) {
    if (!s) return s;
    char* end = s + strlen(s);
    while (end > s && path_is_sep(*(end - 1))) end--;
    *end = '\0';
    return s;
}

// Find the last separator in the string, or NULL if none.
static char* path_find_last_sep(const char* s) {
    if (!s) return NULL;
    char* last = NULL;
    for (const char* p = s; *p; p++) {
        if (path_is_sep(*p)) last = (char*)p;
    }
    return last;
}

// Find the last dot (for extension), or NULL if none.
static char* path_find_last_dot(const char* s) {
    if (!s) return NULL;
    char* last = NULL;
    for (const char* p = s; *p; p++) {
        if (*p == '.') last = (char*)p;
    }
    return last;
}

// Count separators in the string.
static int path_count_seps(const char* s) {
    int count = 0;
    for (const char* p = s; *p; p++) {
        if (path_is_sep(*p)) count++;
    }
    return count;
}

// Copy a string segment to a newly allocated buffer.
static char* path_strndup(const char* s, int64_t len) {
    if (!s) return runtime_str_copy("");
    char* buf = (char*)malloc(len + 1);
    if (!buf) runtime_panic("path: out of memory");
    memcpy(buf, s, len);
    buf[len] = '\0';
    return buf;
}

// -- path_join(a, b) --
// Joins two path components. If b is absolute, returns b.
// Handles empty components gracefully.
char* runtime_path_join(char* a, char* b) {
    if (!a || !*a) return runtime_str_copy(b ? b : "");
    if (!b || !*b) return runtime_str_copy(a);

    // If b starts with / or \, or is a drive letter (C:\), treat as absolute
    if (path_is_sep(b[0])) return runtime_str_copy(b);
    if (strlen(b) >= 2 && b[1] == ':') return runtime_str_copy(b);

    // Find where b's content starts (skip leading separators)
    const char* b_start = b;
    while (*b_start && path_is_sep(*b_start)) b_start++;
    if (!*b_start) return runtime_str_copy(a);

    int a_len = strlen(a);
    char* result = (char*)malloc(a_len + 1 + strlen(b_start) + 1);
    if (!result) runtime_panic("path: out of memory");
    memcpy(result, a, a_len);
    result[a_len] = path_is_sep(a[a_len - 1]) ? '\0' : '/';
    strcat(result + a_len + (result[a_len] == '/' ? 0 : 1), b_start);
    // Fix the separator
    if (result[a_len] != '/') {
        result[a_len] = '/';
    }
    return result;
}

// -- path_basename(path) --
// Returns the last component of a path (after the last separator).
// Strips trailing separators before extracting.
char* runtime_path_basename(char* path) {
    if (!path || !*path) return runtime_str_copy("");

    // Work on a copy to strip trailing separators
    char* copy = runtime_str_copy(path);
    path_skip_trailing_seps(copy);

    char* sep = path_find_last_sep(copy);
    if (!sep) return copy; // No separator, entire string is basename
    char* result = runtime_str_copy(sep + 1);
    free(copy);
    return result;
}

// -- path_dirname(path) --
// Returns the directory portion of a path (everything before the last separator).
// For absolute paths, always returns at least the root.
char* runtime_path_dirname(char* path) {
    if (!path || !*path) return runtime_str_copy(".");

    char* copy = runtime_str_copy(path);
    path_skip_trailing_seps(copy);

    char* sep = path_find_last_sep(copy);
    if (!sep) {
        // No separator => current directory
        free(copy);
        return runtime_str_copy(".");
    }

    // Strip trailing separators from the directory portion
    char* end = sep;
    while (end > copy && path_is_sep(*(end - 1))) end--;

    // If we're at the root (all separators), return the root
    if (end == copy && path_is_sep(*copy)) {
        char root[2] = { *copy, '\0' };
        free(copy);
        return runtime_str_copy(root);
    }

    char* result = path_strndup(copy, end - copy);
    free(copy);
    return result;
}

// -- path_filename(path) --
// Returns the filename portion (basename without extension).
// For "foo/bar.txt" returns "bar", for "foo/bar.tar.gz" returns "bar.tar".
char* runtime_path_filename(char* path) {
    char* base = runtime_path_basename(path);
    char* dot = path_find_last_dot(base);
    if (!dot || dot == base) return base; // No dot, or dot at start => full basename

    char* result = path_strndup(base, dot - base);
    free(base);
    return result;
}

// -- path_extension(path) --
// Returns the file extension including the dot.
// For "foo/bar.txt" returns ".txt", for "foo/bar.tar.gz" returns ".gz".
// Returns "" if no extension.
char* runtime_path_extension(char* path) {
    char* base = runtime_path_basename(path);
    char* dot = path_find_last_dot(base);
    if (!dot || dot == base) {
        free(base);
        return runtime_str_copy("");
    }
    char* result = runtime_str_copy(dot);
    free(base);
    return result;
}

// -- path_is_absolute(path) --
// Returns 1 if the path is absolute, 0 otherwise.
// On Unix: starts with /
// On Windows: starts with \ or drive letter (C:\)
int runtime_path_is_absolute(char* path) {
    if (!path || !*path) return 0;
    if (path_is_sep(path[0])) return 1;
    if (strlen(path) >= 2 && path[1] == ':') return 1;
    return 0;
}

// -- path_normalize(path) --
// Normalizes a path by:
// 1. Collapsing multiple separators into one
// 2. Resolving . (current dir)
// 3. Resolving .. (parent dir)
// 4. Removing trailing separators (except root)
char* runtime_path_normalize(char* path) {
    if (!path || !*path) return runtime_str_copy(".");

    // Split into components, resolve . and ..
    int max_parts = path_count_seps(path) + 2;
    char** parts = (char**)malloc(max_parts * sizeof(char*));
    int count = 0;

    char* work = runtime_str_copy(path);
    char* token = strtok(work, "/\\");
    while (token) {
        if (strcmp(token, ".") == 0) {
            // Skip current dir
        } else if (strcmp(token, "..") == 0) {
            // Go up one level
            if (count > 0) {
                free(parts[count - 1]);
                count--;
            }
        } else {
            parts[count++] = runtime_str_copy(token);
        }
        token = strtok(NULL, "/\\");
    }
    free(work);

    // If empty, return "."
    if (count == 0) {
        free(parts);
        return runtime_str_copy(".");
    }

    // Calculate total length
    int64_t total = 0;
    for (int i = 0; i < count; i++) {
        total += strlen(parts[i]);
    }
    total += count; // separators

    // Add room for leading separator if absolute
    int is_abs = path_is_sep(path[0]) || (strlen(path) >= 2 && path[1] == ':');
    if (is_abs) total++;

    char* result = (char*)malloc(total + 1);
    if (!result) runtime_panic("path: out of memory");
    result[0] = '\0';

    if (is_abs) {
        if (path_is_sep(path[0])) {
            strcpy(result, "/");
        } else {
            // Drive letter
            result[0] = path[0];
            result[1] = ':';
            result[2] = '\0';
        }
    }

    for (int i = 0; i < count; i++) {
        if (i > 0 || result[0] != '\0') {
            if (result[strlen(result) - 1] != '/') {
                strcat(result, "/");
            }
        }
        strcat(result, parts[i]);
        free(parts[i]);
    }
    free(parts);

    return result;
}

// -- path_equals(a, b) --
// Compares two paths for logical equality.
// Normalizes both paths before comparing (case-sensitive).
int runtime_path_equals(char* a, char* b) {
    if (!a && !b) return 1;
    if (!a || !b) return 0;

    char* na = runtime_path_normalize(a);
    char* nb = runtime_path_normalize(b);
    int result = runtime_str_equals(na, nb);
    free(na);
    free(nb);
    return result;
}

// -- path_parent(path) --
// Returns the parent directory. Equivalent to dirname but with different
// semantics: returns "" for root paths and single-component paths.
char* runtime_path_parent(char* path) {
    if (!path || !*path) return runtime_str_copy("");

    char* copy = runtime_str_copy(path);
    path_skip_trailing_seps(copy);

    // If it's a root path (just separator or drive letter + sep),
    // parent of root is root itself.
    if (path_is_sep(copy[0]) && !copy[1]) {
        free(copy);
        return runtime_str_copy("/");
    }
    if (strlen(copy) >= 2 && copy[1] == ':' && (!copy[2] || path_is_sep(copy[2]))) {
        free(copy);
        return runtime_str_copy(copy); // drive root: "C:" or "C:\"
    }

    char* sep = path_find_last_sep(copy);
    if (!sep) {
        free(copy);
        return runtime_str_copy(""); // No parent
    }

    // Strip trailing separators
    char* end = sep;
    while (end > copy && path_is_sep(*(end - 1))) end--;

    if (end == copy && path_is_sep(*copy)) {
        char root[2] = { *copy, '\0' };
        free(copy);
        return runtime_str_copy(root);
    }

    char* result = path_strndup(copy, end - copy);
    free(copy);
    return result;
}

// ========================================================================
// OS operations (Phase C-1.9)
// ========================================================================

char* runtime_os_name(void) {
#ifdef _WIN32
    return runtime_str_copy("windows");
#elif defined(__APPLE__)
    return runtime_str_copy("macos");
#elif defined(__linux__)
    return runtime_str_copy("linux");
#elif defined(__FreeBSD__)
    return runtime_str_copy("freebsd");
#elif defined(__unix__)
    return runtime_str_copy("unix");
#else
    return runtime_str_copy("unknown");
#endif
}

char* runtime_os_arch(void) {
#if defined(__x86_64__) || defined(_M_X64)
    return runtime_str_copy("x86_64");
#elif defined(__aarch64__) || defined(_M_ARM64)
    return runtime_str_copy("aarch64");
#elif defined(__i386__) || defined(_M_IX86)
    return runtime_str_copy("x86");
#elif defined(__arm__) || defined(_M_ARM)
    return runtime_str_copy("arm");
#else
    return runtime_str_copy("unknown");
#endif
}

char* runtime_os_platform(void) {
#ifdef _WIN32
    return runtime_str_copy("windows");
#elif defined(__APPLE__)
    return runtime_str_copy("darwin");
#elif defined(__linux__)
    return runtime_str_copy("linux");
#else
    return runtime_str_copy("unknown");
#endif
}

char* runtime_os_hostname(void) {
#ifdef _WIN32
    char buf[256];
    DWORD len = sizeof(buf);
    if (GetComputerNameA(buf, &len)) {
        return runtime_str_copy(buf);
    }
    return runtime_str_copy("");
#else
    char buf[256];
    if (gethostname(buf, sizeof(buf)) == 0) {
        buf[sizeof(buf) - 1] = '\0';
        return runtime_str_copy(buf);
    }
    return runtime_str_copy("");
#endif
}

char* runtime_os_cwd(void) {
#ifdef _WIN32
    char buf[MAX_PATH];
    DWORD len = GetCurrentDirectoryA(MAX_PATH, buf);
    if (len > 0 && len < MAX_PATH) {
        return runtime_str_copy(buf);
    }
    return runtime_str_copy("");
#else
    char buf[4096];
    if (getcwd(buf, sizeof(buf)) != NULL) {
        return runtime_str_copy(buf);
    }
    return runtime_str_copy("");
#endif
}

int runtime_os_set_cwd(char* path) {
    if (!path || !*path) return 0;
#ifdef _WIN32
    return SetCurrentDirectoryA(path) ? 1 : 0;
#else
    return (chdir(path) == 0) ? 1 : 0;
#endif
}

// ========================================================================
// ENV operations (Phase C-1.9)
// ========================================================================

char* runtime_env_get(char* key) {
    if (!key || !*key) return NULL;
#ifdef _WIN32
    char buf[32768];
    DWORD len = GetEnvironmentVariableA(key, buf, sizeof(buf));
    if (len > 0 && len < sizeof(buf)) {
        return runtime_str_copy(buf);
    }
    return NULL;
#else
    char* val = getenv(key);
    if (val != NULL) {
        return runtime_str_copy(val);
    }
    return NULL;
#endif
}

int runtime_env_has(char* key) {
    if (!key || !*key) return 0;
#ifdef _WIN32
    char buf[1];
    DWORD len = GetEnvironmentVariableA(key, buf, 0);
    return (len > 0) ? 1 : 0;
#else
    return (getenv(key) != NULL) ? 1 : 0;
#endif
}

int runtime_env_set(char* key, char* value) {
    if (!key || !*key) return 0;
    if (!value) value = "";
#ifdef _WIN32
    return SetEnvironmentVariableA(key, value) ? 1 : 0;
#else
    return (setenv(key, value, 1) == 0) ? 1 : 0;
#endif
}

int runtime_env_remove(char* key) {
    if (!key || !*key) return 0;
#ifdef _WIN32
    return SetEnvironmentVariableA(key, NULL) ? 1 : 0;
#else
    return (unsetenv(key) == 0) ? 1 : 0;
#endif
}

// env_all stub for codegen parity; interpreter handles natively.
LimeMap runtime_env_all(void) {
    LimeMap empty = { NULL, 0, 0 };
    return empty;
}
