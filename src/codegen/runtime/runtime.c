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
    char buf[64];
    snprintf(buf, sizeof(buf), "%.6f", v);
    // Trim trailing zeros (but keep at least one digit after the point).
    size_t n = strlen(buf);
    while (n > 0 && buf[n - 1] == '0') n--;
    if (n > 0 && buf[n - 1] == '.') n--;
    buf[n] = '\0';
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
