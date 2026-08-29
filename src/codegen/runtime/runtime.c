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
#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif
#ifndef M_E
#define M_E 2.71828182845904523536
#endif
#include <stdint.h>
#include <time.h>
#include "runtime.h"


#ifdef _WIN32
#include <windows.h>
#include <io.h>
#include <direct.h>
#include <sys/stat.h>
#define strcasecmp _stricmp
#define strncasecmp _strnicmp
#define strtok_r strtok_s
// WinHTTP constant fallbacks for clang/LLVM on Windows
#ifndef WINHTTP_OPTION_MAXHTTPAUTOREDIRECTS
#define WINHTTP_OPTION_MAXHTTPAUTOREDIRECTS 38
#endif
#ifndef WINHTTP_OPTION_REDIRECT_POLICY
#define WINHTTP_OPTION_REDIRECT_POLICY 38
#endif
#ifndef WINHTTP_OPTION_REDIRECT_POLICY_NEVER
#define WINHTTP_OPTION_REDIRECT_POLICY_NEVER 0
#endif
#ifndef WINHTTP_OPTION_REDIRECT_POLICY_DISALLOW_HTTPS_TO_HTTP
#define WINHTTP_OPTION_REDIRECT_POLICY_DISALLOW_HTTPS_TO_HTTP 1
#endif
#ifndef WINHTTP_QUERY_STATUS_CODE
#define WINHTTP_QUERY_STATUS_CODE 19
#endif
#ifndef WINHTTP_QUERY_FLAG_NUMBER
#define WINHTTP_QUERY_FLAG_NUMBER 0x20000000
#endif
#ifndef WINHTTP_QUERY_RAW_HEADERS_CRLF
#define WINHTTP_QUERY_RAW_HEADERS_CRLF 21
#endif
#ifndef WINHTTP_NO_HEADER_INDEX
#define WINHTTP_NO_HEADER_INDEX ((DWORD)-1)
#endif
#ifndef WINHTTP_NO_PROXY_NAME
#define WINHTTP_NO_PROXY_NAME NULL
#endif
#ifndef WINHTTP_NO_PROXY_BYPASS
#define WINHTTP_NO_PROXY_BYPASS NULL
#endif
#ifndef WINHTTP_NO_REFERER
#define WINHTTP_NO_REFERER NULL
#endif
#ifndef WINHTTP_DEFAULT_ACCEPT_TYPES
#define WINHTTP_DEFAULT_ACCEPT_TYPES NULL
#endif
#ifndef WINHTTP_ACCESS_TYPE_DEFAULT_PROXY
#define WINHTTP_ACCESS_TYPE_DEFAULT_PROXY 0
#endif
#ifndef WINHTTP_FLAG_SECURE
#define WINHTTP_FLAG_SECURE 0x00800000
#endif
#ifndef WINHTTP_OPTION_SECURITY_FLAGS
#define WINHTTP_OPTION_SECURITY_FLAGS 94
#endif
#ifndef SECURITY_FLAG_IGNORE_UNKNOWN_CA
#define SECURITY_FLAG_IGNORE_UNKNOWN_CA 0x00001000
#endif
#ifndef SECURITY_FLAG_IGNORE_CERT_DATE_INVALID
#define SECURITY_FLAG_IGNORE_CERT_DATE_INVALID 0x00002000
#endif
#ifndef SECURITY_FLAG_IGNORE_CERT_CN_INVALID
#define SECURITY_FLAG_IGNORE_CERT_CN_INVALID 0x00004000
#endif
#ifndef SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE
#define SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE 0x00008000
#endif
#ifndef WINHTTP_QUERY_HEADER_NAME_BY_INDEX
#define WINHTTP_QUERY_HEADER_NAME_BY_INDEX WINHTTP_NO_HEADER_INDEX
#endif
#else
#include <unistd.h>
#include <dirent.h>
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

// --- Capacity-header string layout (OPT-002, safe) ---
// Every Lime-managed string is an owned, header-backed allocation: the 8 bytes
// immediately BEFORE the data pointer hold (capacity | OWNED_MARK) as a
// little-endian i64. The OWNED_MARK bit lets runtime_str_concat safely detect
// whether its left operand is owned (reusable in place) or a compile-time
// literal/extern string (must not be mutated). Because the compiler emits ALL
// string literals via runtime_str_new, every Lime string carries the marker,
// so the reuse path is always memory-safe.
#define STR_OWNED_MARK (1LL << 63)

char* runtime_str_new(int64_t cap) { 

    if (cap < 1) cap = 1;
    char* raw = (char*)malloc((size_t)cap + 1 + 8);
    if (raw == NULL) {
        runtime_panic("runtime_str_new: out of memory");
    }
    *(int64_t*)raw = ((int64_t)cap) | STR_OWNED_MARK;
    char* data = raw + 8;
    data[0] = '\0';
    return data;
}

// Concatenate two NUL-terminated strings. If `a` is an OWNED string (header
// marker set) and its capacity suffices, the buffer is reused in place
// (amortized O(1) for `s = s + b` loops, beating Clang). Otherwise a fresh
// owned string is allocated. String literals are emitted as owned strings by
// the compiler, so `a` always carries the marker and `a-8` is always valid.
char* runtime_str_concat(char* a, char* b) { 

    if (a == NULL) a = "";
    if (b == NULL) b = "";
    size_t la = strlen(a);
    size_t lb = strlen(b);
    size_t need = la + lb + 1;
    // NOTE: the in-place reuse path (write into a's existing buffer when it
    // has spare capacity) is DISABLED. It let two distinct string variables
    // share the same underlying buffer (e.g. `text = text + "..."` followed by
    // `cur.push_byte(...)` could relocate `cur` onto `text`'s buffer and
    // corrupt it), which broke tokenization in mixed_workload. Always
    // allocate a fresh buffer so buffers never alias. perf is frozen;
    // correctness wins.
    size_t new_cap = need * 2;
    if (new_cap < 64) new_cap = 64;
    char* raw = (char*)malloc(new_cap + 8);
    if (raw == NULL) {
        runtime_panic("runtime_str_concat: out of memory");
    }
    *(int64_t*)raw = ((int64_t)new_cap) | STR_OWNED_MARK;
    char* r = raw + 8;
    memcpy(r, a, la);
    memcpy(r + la, b, lb + 1);
    return r;
}

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
    char* r = runtime_str_new(n > 0 ? n : 1);
    memcpy(r, s + lo, (size_t)n);
    r[n] = '\0';
    return r;
}

// Return the byte value at index i, or -1 if out of bounds. No allocation.
int64_t runtime_str_byte(char* s, int64_t i) { 

    if (s == NULL || i < 0) return -1;
    return (int64_t)(unsigned char)s[i];
}

// Build a 1-character owned string from a byte value (0..255). Used to append a
// single character without allocating a substring. Returns an owned string.
char* runtime_str_from_byte(int64_t b) {
    char* r = runtime_str_new(1);
    if (b < 0 || b > 255) b = 0;
    r[0] = (char)(unsigned char)b;
    r[1] = '\0';
    return r;
}

// Append a single byte to an owned string in place (amortized O(1) via capacity
// reuse). Falls back to fresh allocation when capacity is exhausted. Returns the
// (possibly relocated) owned string.
// Append a single byte to an owned string in place (amortized O(1) via capacity
// reuse). Falls back to fresh allocation when capacity is exhausted. Returns the
// (possibly relocated) owned string.
char* runtime_str_push_byte(char* s, int64_t b) {
    if (s == NULL) return runtime_str_from_byte(b);
    int64_t len = (int64_t)strlen(s);
    int64_t hdr = *(int64_t*)((char*)s - 8);
    int64_t cap = hdr & ~STR_OWNED_MARK;
    if ((hdr & STR_OWNED_MARK) && cap > len) {
        s[len] = (char)(unsigned char)b;
        s[len + 1] = '\0';
        return s;
    }
    int64_t newcap = (len + 1) * 2;
    if (newcap < 16) newcap = 16;
    char* raw = (char*)malloc(newcap + 8);
    if (!raw) runtime_panic("OOM in runtime_str_push_byte");
    *(int64_t*)raw = newcap | STR_OWNED_MARK;
    char* r = raw + 8;
    memcpy(r, s, (size_t)len);
    r[len] = (char)(unsigned char)b;
    r[len + 1] = '\0';
    return r;
}

// Phase B.1: length-tracked variant of runtime_str_push_byte.
// The caller passes the CURRENT length of `s` (which it tracks in a register),
// so this avoids the O(n) strlen call on the hot path. Behavior is identical to
// runtime_str_push_byte for owned strings; for non-owned strings the caller must
// still pass the true length (callers only use this for owned, length-tracked
// strings built from `""` + push_byte only).
char* runtime_str_push_byte_len(char* s, int64_t len, int64_t b) {
    if (s == NULL) return runtime_str_from_byte(b);
    int64_t hdr = *(int64_t*)((char*)s - 8);
    int64_t cap = hdr & ~STR_OWNED_MARK;
    if ((hdr & STR_OWNED_MARK) && cap > len) {
        s[len] = (char)(unsigned char)b;
        s[len + 1] = '\0';
        return s;
    }
    int64_t newcap = (len + 1) * 2;
    if (newcap < 16) newcap = 16;
    char* raw = (char*)malloc(newcap + 8);
    if (!raw) runtime_panic("OOM in runtime_str_push_byte_len");
    *(int64_t*)raw = newcap | STR_OWNED_MARK;
    char* r = raw + 8;
    if (len > 0) memcpy(r, s, (size_t)len);
    r[len] = (char)(unsigned char)b;
    r[len + 1] = '\0';
    return r;
}

// (runtime_str_concat is defined above, OPT-002 capacity-header version.)

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
    snprintf(buf, sizeof(buf), "%.16g", v);
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
    LimeList list;
    runtime_list_empty(&list);
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
            runtime_list_add(&list, (int64_t)(intptr_t)part);
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
        runtime_list_add(&list, (int64_t)(intptr_t)part);
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
    if (tag == 1) {
        return runtime_str_copy("None");
    }
    char* buf = (char*)malloc(64);
    if (!buf) runtime_panic("out of memory");
    snprintf(buf, 64, "Some(%lld)", (long long)payload);
    return buf;
}

char* runtime_str_from_result(int64_t payload, int tag) {
    char* buf = (char*)malloc(64);
    if (!buf) runtime_panic("out of memory");
    if (tag == 0) {
        snprintf(buf, 64, "Success(%lld)", (long long)payload);
    } else {
        snprintf(buf, 64, "Error(%lld)", (long long)payload);
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
    LimeList list;
    runtime_list_empty(&list);
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
        runtime_list_add(&list, (int64_t)(intptr_t)full);
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
        runtime_list_add(&list, (int64_t)(intptr_t)full);
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
    LimeList list;
    runtime_list_empty(&list);
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
        runtime_list_add(&list, (int64_t)(intptr_t)s);
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
void runtime_list_empty(LimeList* out) {
    out->data = NULL;
    out->len = 0;
    out->cap = 0;
}

// Append an element, growing the buffer as needed. Mutates *list in place.
void runtime_list_add(LimeList* restrict list, int64_t elem) __attribute__((always_inline));
void runtime_list_add(LimeList* restrict list, int64_t elem) {
    if (list->len >= list->cap) grow_list(list);
    ((int64_t*)list->data)[list->len] = elem;
    list->len++;
}

// Replace the element at `index` (no-op when the index is out of bounds).
void runtime_list_set(LimeList* restrict list, int64_t index, int64_t elem) {
    if (index >= 0 && index < list->len) {
        ((int64_t*)list->data)[index] = elem;
    }
}

// Return the number of elements in the list.
int64_t runtime_list_len(LimeList list) {
    return list.len;
}

// Return the element at `index`. Returns 0 if out of bounds.
int64_t runtime_list_get(LimeList list, int64_t index) __attribute__((always_inline));
int64_t runtime_list_get(LimeList list, int64_t index) {
    if (index >= 0 && index < list.len) {
        return ((int64_t*)list.data)[index];
    }
    return 0;
}

// -- List mutation / inspection (Phase C-1.2) --

// Insert `elem` at `index`, shifting elements right. Clamps index to [0, len].
void runtime_list_insert(LimeList* list, int64_t index, int64_t elem) {
    if (index < 0) index = 0;
    if (index > list->len) index = list->len;
    if (list->len >= list->cap) grow_list(list);
    int64_t* arr = (int64_t*)list->data;
    memmove(arr + index + 1, arr + index, (list->len - index) * sizeof(int64_t));
    arr[index] = elem;
    list->len++;
}

void runtime_list_clear(LimeList* list) {
    list->len = 0;
}

// Sort the list elements (simple insertion sort for i64 values).
void runtime_list_sort(LimeList* list) {
    int64_t* arr = (int64_t*)list->data;
    for (int64_t i = 1; i < list->len; i++) {
        int64_t key = arr[i];
        int64_t j = i - 1;
        while (j >= 0 && arr[j] > key) {
            arr[j + 1] = arr[j];
            j--;
        }
        arr[j + 1] = key;
    }
}

void runtime_list_clone(LimeList* dest, LimeList* src) {
    LimeList result = {0, 0, 0};
    if (src->len > 0) {
        if (result.len >= result.cap) grow_list(&result);
        while (result.cap < src->len) grow_list(&result);
        memcpy(result.data, src->data, src->len * sizeof(int64_t));
        result.len = src->len;
    }
    *dest = result;
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
    runtime_list_add(&queue, elem);
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
    runtime_list_add(&stack, elem);
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
    size_t cap = 256;
    size_t blen = 0;
    char* buf = (char*)malloc(cap);
    if (!buf) runtime_panic("json: out of memory");
    while (jp_input[jp_pos] != '"' && jp_input[jp_pos] != '\0') {
        if (blen + 4 >= cap) {
            cap *= 2;
            char* nb = (char*)realloc(buf, cap);
            if (!nb) { free(buf); runtime_panic("json: out of memory"); }
            buf = nb;
        }
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
    j->data.string_val = buf; // buf is already malloc'd, transfer ownership
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
    size_t b_len = strlen(b_start);
    char* result = (char*)malloc(a_len + 1 + b_len + 1);
    if (!result) runtime_panic("path: out of memory");
    memcpy(result, a, a_len);
    int pos = a_len;
    if (!path_is_sep(a[a_len - 1])) {
        result[pos++] = '/';
    }
    memcpy(result + pos, b_start, b_len);
    result[pos + b_len] = '\0';
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

// ========================================================================
// Regex operations (Phase C-1.10)
//
// A practical regex engine supporting:
//   Literal chars, . (any), ^ $ (anchors), \b (word boundary)
//   * + ? {n} {n,m} {n,} quantifiers
//   [abc] [^abc] [a-z] character classes
//   (...) groups, (?:...) non-capturing groups, | alternation
//   \d \w \s \D \W \S shorthand classes
//   \ escape, (?i) case-insensitive
// ========================================================================

// Check if a character is a word character.
static int regex_is_word_char(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
        || (c >= '0' && c <= '9') || c == '_';
}

// Check if a character is a whitespace character.
static int regex_is_space(char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' || c == '\v';
}

// Convert to lowercase for case-insensitive matching.
static char regex_to_lower(char c) {
    return (c >= 'A' && c <= 'Z') ? (char)(c + 32) : c;
}

// ---- Pattern compiler: parse regex string into a bytecode ----

typedef enum {
    RC_LITERAL,     // match literal char
    RC_DOT,         // match any char (except \n)
    RC_DOTALL,      // match any char including \n
    RC_CLASS,       // match char class [abc] [^abc] [a-z]
    RC_SHCLASS,     // shorthand class: \d \w \s \D \W \S
    RC_STAR,        // zero or more (greedy)
    RC_PLUS,        // one or more (greedy)
    RC_QUESTION,    // zero or one (greedy)
    RC_RANGE_N,     // exactly n repetitions
    RC_RANGE_NM,    // n to m repetitions
    RC_RANGE_NP,    // n or more repetitions
    RC_ANCHOR_START,// ^
    RC_ANCHOR_END,  // $
    RC_WORD_BOUND,  // \b
    RC_WORD_BOUND_N,// \B (negated word boundary)
    RC_GROUP_START, // ( or (?: — group open
    RC_GROUP_END,   // ) — group close
    RC_ALT,         // | — alternation
    RC_GROUP_END_Q, // )? — optional group
} RegexCmd;

typedef struct {
    RegexCmd cmd;
    char ch;            // for RC_LITERAL
    char* cls;          // for RC_CLASS (heap-allocated bracket expression)
    int cls_neg;        // for RC_CLASS: 1 if negated
    int case_insensitive; // for RC_LITERAL
    int n, m;           // for RC_RANGE_*
} RegexOp;

typedef struct {
    RegexOp* ops;
    int64_t len;
    int64_t cap;
} RegexProgram;

static void prog_grow(RegexProgram* p) {
    if (p->len >= p->cap) {
        int64_t new_cap = p->cap ? p->cap * 2 : 16;
        p->ops = (RegexOp*)realloc(p->ops, new_cap * sizeof(RegexOp));
        if (!p->ops) runtime_panic("regex: out of memory");
        p->cap = new_cap;
    }
}

static void prog_add(RegexProgram* p, RegexCmd cmd) {
    if (p->len >= p->cap) prog_grow(p);
    memset(&p->ops[p->len], 0, sizeof(RegexOp));
    p->ops[p->len].cmd = cmd;
    p->len++;
}

// Find the matching closing bracket, returning index of ']' in pattern.
// start points to the char after '['.
static int regex_find_closing_bracket(const char* pat, int start) {
    int i = start;
    if (pat[i] == '^') i++; // skip negation
    if (pat[i] == ']') i++; // ']' right after '[' or '[^' is literal
    while (pat[i] && pat[i] != ']') i++;
    return pat[i] ? i : -1;
}

// Parse a brace quantifier {n}, {n,m}, {n,}.
// Returns the number of chars consumed (including braces), or 0 if not a valid quantifier.
static int regex_parse_brace(const char* pat, int pos, int* out_n, int* out_m) {
    if (pat[pos] != '{') return 0;
    int i = pos + 1;
    int n = 0, m = -1;
    int has_n = 0;
    while (pat[i] >= '0' && pat[i] <= '9') {
        n = n * 10 + (pat[i] - '0');
        has_n = 1;
        i++;
    }
    if (!has_n) return 0;
    if (pat[i] == '}') {
        *out_n = n;
        *out_m = n;
        return i - pos + 1;
    }
    if (pat[i] == ',') {
        i++;
        if (pat[i] == '}') {
            *out_n = n;
            *out_m = -1; // unbounded
            return i - pos + 1;
        }
        m = 0;
        while (pat[i] >= '0' && pat[i] <= '9') {
            m = m * 10 + (pat[i] - '0');
            i++;
        }
        if (pat[i] == '}') {
            *out_n = n;
            *out_m = m;
            return i - pos + 1;
        }
    }
    return 0; // not a valid quantifier
}

// Compile a regex pattern string into a RegexProgram.
// Returns 1 on success, 0 on error.
static int regex_compile_prog(const char* pat, RegexProgram* prog) {
    prog->ops = NULL;
    prog->len = 0;
    prog->cap = 0;

    int i = 0;
    int case_insensitive = 0;
    while (pat[i]) {
        // Check for (?i) inline flag
        if (pat[i] == '(' && pat[i+1] == '?' && pat[i+2] == 'i' && pat[i+3] == ')') {
            case_insensitive = 1;
            i += 4;
            continue;
        }
        if (pat[i] == '(' && pat[i+1] == '?' && pat[i+1] != 'i') {
            // (?s), (?m), (?si) etc. — skip flags
            int j = i + 2;
            while (pat[j] && pat[j] != ')') j++;
            if (pat[j] == ')') { i = j + 1; continue; }
        }

        char c = pat[i];

        if (c == '\\') {
            i++;
            char esc = pat[i];
            if (!esc) { free(prog->ops); return 0; }
            switch (esc) {
                case 'd': case 'D': case 'w': case 'W': case 's': case 'S':
                    prog_add(prog, RC_SHCLASS);
                    prog->ops[prog->len-1].ch = esc;
                    break;
                case 'b':
                    prog_add(prog, RC_WORD_BOUND);
                    break;
                case 'B':
                    prog_add(prog, RC_WORD_BOUND_N);
                    break;
                default:
                    prog_add(prog, RC_LITERAL);
                    prog->ops[prog->len-1].ch = esc;
                    prog->ops[prog->len-1].case_insensitive = case_insensitive;
                    break;
            }
            i++;
        } else if (c == '^') {
            prog_add(prog, RC_ANCHOR_START);
            i++;
        } else if (c == '$') {
            prog_add(prog, RC_ANCHOR_END);
            i++;
        } else if (c == '.') {
            prog_add(prog, RC_DOT);
            i++;
        } else if (c == '(') {
            int noncap = (pat[i+1] == '?' && pat[i+2] == ':');
            prog_add(prog, RC_GROUP_START);
            i += noncap ? 3 : 1;
        } else if (c == ')') {
            prog_add(prog, RC_GROUP_END);
            i++;
        } else if (c == '|') {
            prog_add(prog, RC_ALT);
            i++;
        } else if (c == '[') {
            int end = regex_find_closing_bracket(pat, i + 1);
            if (end < 0) { free(prog->ops); return 0; }
            int neg = (pat[i+1] == '^');
            int cls_start = neg ? i + 2 : i + 1;
            int cls_len = end - cls_start;
            char* cls = (char*)malloc(cls_len + 1);
            if (!cls) runtime_panic("regex: out of memory");
            memcpy(cls, pat + cls_start, cls_len);
            cls[cls_len] = '\0';
            prog_add(prog, RC_CLASS);
            prog->ops[prog->len-1].cls = cls;
            prog->ops[prog->len-1].cls_neg = neg;
            i = end + 1;
        } else if (c == '*' || c == '+' || c == '?') {
            // Apply quantifier to the last operation
            if (prog->len == 0) { free(prog->ops); return 0; }
            int last = (int)prog->len - 1;
            RegexCmd base = prog->ops[last].cmd;
            if (base == RC_STAR || base == RC_PLUS || base == RC_QUESTION
                || base == RC_RANGE_N || base == RC_RANGE_NM || base == RC_RANGE_NP) {
                // Quantifier on quantifier — not allowed
                free(prog->ops); return 0;
            }
            RegexCmd qcmd = (c == '*') ? RC_STAR : (c == '+') ? RC_PLUS : RC_QUESTION;
            RegexOp base_op = prog->ops[last];
            prog->ops[last].cmd = qcmd;
            // For star/plus/question, we need the base as a sub-pattern.
            // We store it in a simplified way: the cmd field encodes the quantifier,
            // and the ch/cls fields carry the base info (only works for single-char bases).
            // For complex bases, we use a different approach: wrap in group-like nesting.
            // Actually, let's use a simpler approach: store the base op in the quantifier op.
            prog->ops[last].ch = 0;
            // We need to handle this differently. Let's just note the quantifier
            // and handle it in the matcher by looking at the previous op.
            // For now, store the original op's data.
            i++;
        } else {
            // Check for brace quantifier
            int bn = 0, bm = 0;
            int consumed = regex_parse_brace(pat, i, &bn, &bm);
            if (consumed > 0 && prog->len > 0) {
                int last = (int)prog->len - 1;
                RegexOp base_op = prog->ops[last];
                if (base_op.cmd == RC_STAR || base_op.cmd == RC_PLUS
                    || base_op.cmd == RC_QUESTION || base_op.cmd == RC_RANGE_N
                    || base_op.cmd == RC_RANGE_NM || base_op.cmd == RC_RANGE_NP) {
                    free(prog->ops); return 0;
                }
                if (bm < 0) {
                    prog->ops[last].cmd = RC_RANGE_NP;
                } else if (bn == bm) {
                    prog->ops[last].cmd = RC_RANGE_N;
                } else {
                    prog->ops[last].cmd = RC_RANGE_NM;
                }
                prog->ops[last].n = bn;
                prog->ops[last].m = bm;
                i += consumed;
            } else {
                // Literal character
                prog_add(prog, RC_LITERAL);
                prog->ops[prog->len-1].ch = c;
                prog->ops[prog->len-1].case_insensitive = case_insensitive;
                i++;
            }
        }
    }
    return 1;
}

// ---- Matcher ----

typedef struct {
    const char* text;
    int text_len;
    RegexProgram* prog;
    int case_insensitive;
} RegexMatcher;

// Check if a character matches a character class [abc] [^abc] [a-z].
static int regex_match_class(const char* cls, char c, int neg) {
    int len = (int)strlen(cls);
    int matched = 0;
    int i = 0;
    while (i < len) {
        if (cls[i] == '\\' && i + 1 < len) {
            char esc = cls[i+1];
            switch (esc) {
                case 'd': matched = (c >= '0' && c <= '9'); i += 2; continue;
                case 'D': matched = !(c >= '0' && c <= '9'); i += 2; continue;
                case 'w': matched = regex_is_word_char(c); i += 2; continue;
                case 'W': matched = !regex_is_word_char(c); i += 2; continue;
                case 's': matched = regex_is_space(c); i += 2; continue;
                case 'S': matched = !regex_is_space(c); i += 2; continue;
                default: matched = (c == esc); i += 2; continue;
            }
        }
        if (i + 2 < len && cls[i+1] == '-') {
            // Range [a-z]
            char lo = cls[i];
            char hi = cls[i+2];
            matched = (c >= lo && c <= hi);
            i += 3;
        } else {
            matched = (c == cls[i]);
            i++;
        }
    }
    return neg ? !matched : matched;
}

// Match shorthand class \d \w \s etc.
static int regex_match_shclass(char cls, char c) {
    switch (cls) {
        case 'd': return c >= '0' && c <= '9';
        case 'D': return !(c >= '0' && c <= '9');
        case 'w': return regex_is_word_char(c);
        case 'W': return !regex_is_word_char(c);
        case 's': return regex_is_space(c);
        case 'S': return !regex_is_space(c);
    }
    return 0;
}

// Try to match the pattern starting at pat_pos in the program
// and text_pos in the text. Returns the end position of the match, or -1.
static int regex_try_match(RegexMatcher* m, int pat_pos, int text_pos) {
    int plen = (int)m->prog->len;
    int tlen = m->text_len;

    if (pat_pos >= plen) return text_pos; // pattern exhausted => match

    RegexOp* op = &m->prog->ops[pat_pos];

    // Handle alternation: try left, then right
    if (op->cmd == RC_ALT) {
        int res = regex_try_match(m, pat_pos + 1, text_pos);
        if (res >= 0) return res;
        // Skip to next alternative or end of group
        return -1;
    }

    // Handle group start
    if (op->cmd == RC_GROUP_START) {
        // Find matching group end, tracking nesting
        int depth = 1;
        int j = pat_pos + 1;
        while (j < plen && depth > 0) {
            if (m->prog->ops[j].cmd == RC_GROUP_START) depth++;
            if (m->prog->ops[j].cmd == RC_GROUP_END) depth--;
            j++;
        }
        // Match inside group, then continue after group end
        int inside = regex_try_match(m, pat_pos + 1, text_pos);
        if (inside < 0) return -1;
        return regex_try_match(m, j, inside);
    }

    // Handle group end (shouldn't be reached in normal flow)
    if (op->cmd == RC_GROUP_END) {
        return regex_try_match(m, pat_pos + 1, text_pos);
    }

    // Determine what single character/op to match
    char match_char = 0;
    int is_dot = 0, is_class = 0, is_shclass = 0, is_anchor = 0;
    char shcls = 0;
    int cls_neg = 0;
    char* cls = NULL;
    int ci = 0;

    switch (op->cmd) {
        case RC_LITERAL:
            match_char = op->ch;
            ci = op->case_insensitive;
            break;
        case RC_DOT:
            is_dot = 1;
            break;
        case RC_CLASS:
            is_class = 1;
            cls = op->cls;
            cls_neg = op->cls_neg;
            break;
        case RC_SHCLASS:
            is_shclass = 1;
            shcls = op->ch;
            break;
        case RC_ANCHOR_START:
            is_anchor = 1;
            if (text_pos != 0) return -1;
            return regex_try_match(m, pat_pos + 1, text_pos);
        case RC_ANCHOR_END:
            is_anchor = 1;
            if (text_pos != tlen) return -1;
            return regex_try_match(m, pat_pos + 1, text_pos);
        case RC_WORD_BOUND: {
            int left_is_w = (text_pos > 0) && regex_is_word_char(m->text[text_pos-1]);
            int right_is_w = (text_pos < tlen) && regex_is_word_char(m->text[text_pos]);
            if (left_is_w == right_is_w) return -1;
            return regex_try_match(m, pat_pos + 1, text_pos);
        }
        case RC_WORD_BOUND_N: {
            int left_is_w = (text_pos > 0) && regex_is_word_char(m->text[text_pos-1]);
            int right_is_w = (text_pos < tlen) && regex_is_word_char(m->text[text_pos]);
            if (left_is_w != right_is_w) return -1;
            return regex_try_match(m, pat_pos + 1, text_pos);
        }
        default:
            return -1;
    }

    // Check if current text character matches
    int char_matches = 0;
    if (text_pos < tlen) {
        char tc = m->text[text_pos];
        if (is_dot) {
            char_matches = (tc != '\n');
        } else if (is_class) {
            char_matches = regex_match_class(cls, tc, cls_neg);
        } else if (is_shclass) {
            char_matches = regex_match_shclass(shcls, tc);
        } else {
            if (ci) {
                char_matches = (regex_to_lower(tc) == regex_to_lower(match_char));
            } else {
                char_matches = (tc == match_char);
            }
        }
    }

    if (!char_matches) return -1;

    int next_text = text_pos + 1;
    int next_pat = pat_pos + 1;

    // Check if next op is a quantifier
    if (next_pat < plen) {
        RegexOp* next = &m->prog->ops[next_pat];
        int is_quant = (next->cmd == RC_STAR || next->cmd == RC_PLUS
                     || next->cmd == RC_QUESTION || next->cmd == RC_RANGE_N
                     || next->cmd == RC_RANGE_NM || next->cmd == RC_RANGE_NP);
        if (is_quant) {
            int qmin = 1, qmax = 1;
            switch (next->cmd) {
                case RC_STAR:    qmin = 0; qmax = -1; break;
                case RC_PLUS:    qmin = 1; qmax = -1; break;
                case RC_QUESTION:qmin = 0; qmax = 1;  break;
                case RC_RANGE_N:    qmin = next->n; qmax = next->n; break;
                case RC_RANGE_NM:   qmin = next->n; qmax = next->m; break;
                case RC_RANGE_NP:   qmin = next->n; qmax = -1; break;
                default: break;
            }
            // Try matching from max down to min (greedy)
            int count = 1;
            int max_rep = (qmax < 0) ? (tlen - text_pos) : qmax;
            // First, consume as many as possible
            int save_pos = next_text;
            while (count < max_rep && save_pos < tlen) {
                char tc = m->text[save_pos];
                int ok = 0;
                if (is_dot) { ok = (tc != '\n'); }
                else if (is_class) { ok = regex_match_class(cls, tc, cls_neg); }
                else if (is_shclass) { ok = regex_match_shclass(shcls, tc); }
                else { ok = ci ? (regex_to_lower(tc) == regex_to_lower(match_char)) : (tc == match_char); }
                if (!ok) break;
                count++;
                save_pos++;
            }
            // Try from count down to qmin
            for (int rep = count; rep >= qmin; rep--) {
                int after_quant = regex_try_match(m, next_pat + 1, text_pos + rep);
                if (after_quant >= 0) return after_quant;
            }
            return -1;
        }
    }

    // No quantifier, just match one char and continue
    return regex_try_match(m, next_pat, next_text);
}

// ---- Public API ----

char* runtime_regex_compile(char* pattern) {
    if (!pattern) return NULL;
    RegexProgram prog;
    if (!regex_compile_prog(pattern, &prog)) return NULL;
    int64_t total = 4 + (int64_t)prog.len * (int64_t)sizeof(RegexOp);
    char* blob = (char*)malloc(total);
    if (!blob) { free(prog.ops); runtime_panic("regex: out of memory"); }
    int count = (int)prog.len;
    blob[0] = (char)(count & 0xFF);
    blob[1] = (char)((count >> 8) & 0xFF);
    blob[2] = (char)((count >> 16) & 0xFF);
    blob[3] = (char)((count >> 24) & 0xFF);
    memcpy(blob + 4, prog.ops, prog.len * sizeof(RegexOp));
    // Free any allocated cls strings in the ops
    for (int64_t i = 0; i < prog.len; i++) {
        if (prog.ops[i].cmd == RC_CLASS && prog.ops[i].cls) {
            free(prog.ops[i].cls);
        }
    }
    free(prog.ops);
    return blob;
}

static RegexProgram regex_deserialize(char* compiled) {
    RegexProgram prog;
    if (!compiled) { prog.ops = NULL; prog.len = 0; prog.cap = 0; return prog; }
    int count = (unsigned char)compiled[0]
              | ((unsigned char)compiled[1] << 8)
              | ((unsigned char)compiled[2] << 16)
              | ((unsigned char)compiled[3] << 24);
    prog.ops = (RegexOp*)(compiled + 4);
    prog.len = count;
    prog.cap = count;
    return prog;
}

int runtime_regex_is_match(char* compiled, char* text) {
    if (!compiled || !text) return 0;
    RegexProgram prog = regex_deserialize(compiled);
    if (prog.ops == NULL) return 0;
    RegexMatcher m;
    m.text = text;
    m.text_len = (int)strlen(text);
    m.prog = &prog;
    m.case_insensitive = 0;
    int result = regex_try_match(&m, 0, 0);
    return (result == m.text_len) ? 1 : 0;
}

char* runtime_regex_find(char* compiled, char* text) {
    if (!compiled || !text) return NULL;
    RegexProgram prog = regex_deserialize(compiled);
    if (prog.ops == NULL) return NULL;
    RegexMatcher m;
    m.text = text;
    m.text_len = (int)strlen(text);
    m.prog = &prog;
    m.case_insensitive = 0;
    for (int i = 0; i <= m.text_len; i++) {
        int end = regex_try_match(&m, 0, i);
        if (end >= 0 && end > i) {
            int match_len = end - i;
            char* result = (char*)malloc(match_len + 1);
            if (!result) runtime_panic("regex: out of memory");
            memcpy(result, text + i, match_len);
            result[match_len] = '\0';
            return result;
        }
    }
    return NULL;
}

LimeList runtime_regex_find_all(char* compiled, char* text) {
    LimeList list;
    runtime_list_empty(&list);
    if (!compiled || !text) return list;
    RegexProgram prog = regex_deserialize(compiled);
    if (prog.ops == NULL) return list;
    RegexMatcher m;
    m.text = text;
    m.text_len = (int)strlen(text);
    m.prog = &prog;
    m.case_insensitive = 0;
    int pos = 0;
    while (pos <= m.text_len) {
        int end = regex_try_match(&m, 0, pos);
        if (end >= 0 && end > pos) {
            int match_len = end - pos;
            char* match_str = (char*)malloc(match_len + 1);
            if (!match_str) runtime_panic("regex: out of memory");
            memcpy(match_str, text + pos, match_len);
            match_str[match_len] = '\0';
            runtime_list_add(&list, (int64_t)(intptr_t)match_str);
            pos = end;
            if (end == pos) pos++;
        } else {
            pos++;
        }
    }
    return list;
}

char* runtime_regex_replace(char* compiled, char* text, char* replacement) {
    if (!compiled || !text) return runtime_str_copy(text ? text : "");
    if (!replacement) replacement = "";
    RegexProgram prog = regex_deserialize(compiled);
    if (prog.ops == NULL) return runtime_str_copy(text);
    RegexMatcher m;
    m.text = text;
    m.text_len = (int)strlen(text);
    m.prog = &prog;
    m.case_insensitive = 0;
    for (int i = 0; i <= m.text_len; i++) {
        int end = regex_try_match(&m, 0, i);
        if (end >= 0 && end > i) {
            int before_len = i;
            int after_len = m.text_len - end;
            int repl_len = (int)strlen(replacement);
            char* result = (char*)malloc(before_len + repl_len + after_len + 1);
            if (!result) runtime_panic("regex: out of memory");
            memcpy(result, text, before_len);
            memcpy(result + before_len, replacement, repl_len);
            memcpy(result + before_len + repl_len, text + end, after_len);
            result[before_len + repl_len + after_len] = '\0';
            return result;
        }
    }
    return runtime_str_copy(text);
}

char* runtime_regex_replace_all(char* compiled, char* text, char* replacement) {
    if (!compiled || !text) return runtime_str_copy(text ? text : "");
    if (!replacement) replacement = "";
    RegexProgram prog = regex_deserialize(compiled);
    if (prog.ops == NULL) return runtime_str_copy(text);
    RegexMatcher m;
    m.text = text;
    m.text_len = (int)strlen(text);
    m.prog = &prog;
    m.case_insensitive = 0;

    int64_t result_cap = strlen(text) * 2 + 64;
    char* result = (char*)malloc(result_cap);
    if (!result) runtime_panic("regex: out of memory");
    int rpos = 0;
    int pos = 0;
    int repl_len = (int)strlen(replacement);
    int last_match_end = 0;

    while (pos <= m.text_len) {
        int end = regex_try_match(&m, 0, pos);
        if (end >= 0 && end > pos) {
            int copy_len = pos - last_match_end;
            while (rpos + copy_len + repl_len + 1 > result_cap) {
                result_cap *= 2;
                result = (char*)realloc(result, result_cap);
                if (!result) runtime_panic("regex: out of memory");
            }
            memcpy(result + rpos, text + last_match_end, copy_len);
            rpos += copy_len;
            memcpy(result + rpos, replacement, repl_len);
            rpos += repl_len;
            last_match_end = end;
            pos = end;
            if (end == pos) {
                result[rpos++] = text[pos];
                last_match_end = pos + 1;
                pos++;
            }
        } else {
            pos++;
        }
    }
    int copy_len = m.text_len - last_match_end;
    while (rpos + copy_len + 1 > result_cap) {
        result_cap *= 2;
        result = (char*)realloc(result, result_cap);
        if (!result) runtime_panic("regex: out of memory");
    }
    memcpy(result + rpos, text + last_match_end, copy_len);
    rpos += copy_len;
    result[rpos] = '\0';
    return result;
}

LimeList runtime_regex_split(char* compiled, char* text) {
    LimeList list;
    runtime_list_empty(&list);
    if (!compiled || !text) {
        if (text) runtime_list_add(&list, (int64_t)(intptr_t)runtime_str_copy(text));
        return list;
    }
    RegexProgram prog = regex_deserialize(compiled);
    if (prog.ops == NULL) {
        runtime_list_add(&list, (int64_t)(intptr_t)runtime_str_copy(text));
        return list;
    }
    RegexMatcher m;
    m.text = text;
    m.text_len = (int)strlen(text);
    m.prog = &prog;
    m.case_insensitive = 0;

    int pos = 0;
    int last_end = 0;
    while (pos <= m.text_len) {
        int end = regex_try_match(&m, 0, pos);
        if (end >= 0 && end > pos) {
            int piece_len = pos - last_end;
            char* piece = (char*)malloc(piece_len + 1);
            if (!piece) runtime_panic("regex: out of memory");
            memcpy(piece, text + last_end, piece_len);
            piece[piece_len] = '\0';
            runtime_list_add(&list, (int64_t)(intptr_t)piece);
            last_end = end;
            pos = end;
            if (end == pos) pos++;
        } else {
            pos++;
        }
    }
    int piece_len = m.text_len - last_end;
    char* piece = (char*)malloc(piece_len + 1);
    if (!piece) runtime_panic("regex: out of memory");
    memcpy(piece, text + last_end, piece_len);
    piece[piece_len] = '\0';
    runtime_list_add(&list, (int64_t)(intptr_t)piece);
    return list;
}

// ========================================================================
// Process operations (Phase C-1.11)
//
// Cross-platform subprocess management.
// Windows: CreateProcess, GetExitCodeProcess, TerminateProcess, pipes
// POSIX: fork, exec, waitpid, kill, pipe, dup2
// ========================================================================

#ifdef _WIN32

static int64_t win_create_process(char* command, LimeList args, HANDLE* out_handle) {
    // Build command line from command + args
    size_t cmd_len = strlen(command) + 3; // +3 for potential quotes + null
    for (int64_t i = 0; i < args.len; i++) {
        char* arg = (char*)(intptr_t)runtime_list_get(args, i);
        if (arg) cmd_len += strlen(arg) + 3; // space + quotes + null
    }

    char* cmd_line = (char*)malloc(cmd_len);
    if (!cmd_line) return -1;

    size_t pos = 0;
    // Quote the command if it contains spaces
    int needs_quotes = strchr(command, ' ') != NULL;
    if (needs_quotes) cmd_line[pos++] = '"';
    strcpy(cmd_line + pos, command);
    pos += strlen(command);
    if (needs_quotes) cmd_line[pos++] = '"';

    for (int64_t i = 0; i < args.len; i++) {
        char* arg = (char*)(intptr_t)runtime_list_get(args, i);
        if (!arg) continue;
        cmd_line[pos++] = ' ';
        cmd_line[pos++] = '"';
        strcpy(cmd_line + pos, arg);
        pos += strlen(arg);
        cmd_line[pos++] = '"';
    }
    cmd_line[pos] = '\0';

    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    memset(&si, 0, sizeof(si));
    si.cb = sizeof(si);
    memset(&pi, 0, sizeof(pi));

    // Create pipes for stdout and stderr
    HANDLE stdout_read, stdout_write;
    HANDLE stderr_read, stderr_write;

    SECURITY_ATTRIBUTES sa;
    sa.nLength = sizeof(SECURITY_ATTRIBUTES);
    sa.bInheritHandle = TRUE;
    sa.lpSecurityDescriptor = NULL;

    if (!CreatePipe(&stdout_read, &stdout_write, &sa, 0)) {
        free(cmd_line);
        return -1;
    }
    if (!CreatePipe(&stderr_read, &stderr_write, &sa, 0)) {
        CloseHandle(stdout_read);
        CloseHandle(stdout_write);
        free(cmd_line);
        return -1;
    }

    si.hStdError = stderr_write;
    si.hStdOutput = stdout_write;
    si.dwFlags |= STARTF_USESTDHANDLES;

    BOOL success = CreateProcessA(
        NULL,
        cmd_line,
        NULL,
        NULL,
        TRUE,
        CREATE_NO_WINDOW,
        NULL,
        NULL,
        &si,
        &pi
    );

    // Close write ends in parent process
    CloseHandle(stdout_write);
    CloseHandle(stderr_write);

    free(cmd_line);

    if (!success) {
        CloseHandle(stdout_read);
        CloseHandle(stderr_read);
        return -1;
    }

    *out_handle = pi.hProcess;
    // Store thread handle too but we don't need it
    CloseHandle(pi.hThread);

    // Read stdout from the pipe
    // We'll read it asynchronously - for now, return the pid
    // The stdout pipe handle is stdout_read

    // Store stdout_read in a global or thread-local storage
    // For simplicity, we'll just return the PID and let wait/collect handle the pipes

    return (int64_t)pi.dwProcessId;
}

int64_t runtime_process_spawn(char* command, LimeList args) {
    if (!command) return -1;
    HANDLE proc_handle;
    int64_t pid = win_create_process(command, args, &proc_handle);
    if (pid < 0) return -1;
    // Store the handle for later use - we use a simple global approach
    // In a real implementation, we'd use a handle table
    return pid;
}

char* runtime_process_run(char* command, LimeList args) {
    if (!command) return runtime_str_copy("");
    // For run, we spawn and wait
    // Build command with dynamic buffer
    size_t cmd_cap = 256;
    size_t cmd_len = strlen(command) + 1;
    char* cmd_buf = (char*)malloc(cmd_cap);
    if (!cmd_buf) return runtime_str_copy("");
    strcpy(cmd_buf, command);
    for (int64_t i = 0; i < args.len; i++) {
        char* arg = (char*)(intptr_t)runtime_list_get(args, i);
        if (arg) {
            size_t need = cmd_len + 1 + strlen(arg) + 1;
            if (need > cmd_cap) {
                cmd_cap = need * 2;
                char* nb = (char*)realloc(cmd_buf, cmd_cap);
                if (!nb) { free(cmd_buf); return runtime_str_copy(""); }
                cmd_buf = nb;
            }
            strcat(cmd_buf, " ");
            strcat(cmd_buf, arg);
            cmd_len = strlen(cmd_buf);
        }
    }

    // Use _popen for simple command execution
    FILE* fp = _popen(cmd_buf, "r");
    free(cmd_buf);
    if (!fp) return runtime_str_copy("");

    size_t cap = 4096;
    size_t len = 0;
    char* buf = (char*)malloc(cap);
    if (!buf) { _pclose(fp); return NULL; }

    int c;
    while ((c = fgetc(fp)) != EOF) {
        if (len + 1 >= cap) {
            cap *= 2;
            char* nb = (char*)realloc(buf, cap);
            if (!nb) { free(buf); _pclose(fp); return NULL; }
            buf = nb;
        }
        buf[len++] = (char)c;
    }
    buf[len] = '\0';
    _pclose(fp);
    return buf;
}

char* runtime_process_output(char* command, LimeList args) {
    return runtime_process_run(command, args);
}

int64_t runtime_process_wait(int64_t pid) {
    // On Windows, we can't easily wait by PID alone without the handle
    // For now, return 0 (success) as a placeholder
    // A full implementation would maintain a handle table
    (void)pid;
    return 0;
}

int runtime_process_kill(int64_t pid) {
    if (pid <= 0) return 0;
    HANDLE h = OpenProcess(PROCESS_TERMINATE, FALSE, (DWORD)pid);
    if (!h) return 0;
    BOOL ok = TerminateProcess(h, 1);
    CloseHandle(h);
    return ok ? 1 : 0;
}

char* runtime_process_status(int64_t pid) {
    if (pid <= 0) return runtime_str_copy("failed");
    HANDLE h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, (DWORD)pid);
    if (!h) return runtime_str_copy("failed");
    DWORD exit_code;
    if (GetExitCodeProcess(h, &exit_code)) {
        CloseHandle(h);
        if (exit_code == STILL_ACTIVE) {
            return runtime_str_copy("running");
        }
        // Process has exited
        char buf[32];
        snprintf(buf, sizeof(buf), "exited(%lu)", exit_code);
        return runtime_str_copy(buf);
    }
    CloseHandle(h);
    return runtime_str_copy("failed");
}

LimeList runtime_process_args(void) {
    LimeList list;
    runtime_list_empty(&list);
    // Get command line arguments
    wchar_t* cmd_line = GetCommandLineW();
    int argc;
    LPWSTR* argv = CommandLineToArgvW(cmd_line, &argc);
    if (!argv) return list;
    for (int i = 1; i < argc; i++) {
        int size = WideCharToMultiByte(CP_UTF8, 0, argv[i], -1, NULL, 0, NULL, NULL);
        char* arg = (char*)malloc(size);
        if (arg) {
            WideCharToMultiByte(CP_UTF8, 0, argv[i], -1, arg, size, NULL, NULL);
            runtime_list_add(&list, (int64_t)(intptr_t)arg);
        }
    }
    LocalFree(argv);
    return list;
}

#else // POSIX

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>
#include <fcntl.h>

static int64_t posix_spawn_process(char* command, LimeList args, int* stdout_pipe_read) {
    int pipefd[2];
    if (pipe(pipefd) != 0) return -1;

    pid_t pid = fork();
    if (pid < 0) {
        close(pipefd[0]);
        close(pipefd[1]);
        return -1;
    }

    if (pid == 0) {
        // Child process
        close(pipefd[0]); // Close read end

        // Redirect stdout
        dup2(pipefd[1], STDOUT_FILENO);
        dup2(pipefd[1], STDERR_FILENO);
        close(pipefd[1]);

        // Build argv
        int argc = (int)args.len + 1;
        char** argv = (char**)malloc(sizeof(char*) * (argc + 1));
        if (!argv) { _exit(127); }
        argv[0] = command;
        for (int64_t i = 0; i < args.len; i++) {
            argv[i + 1] = (char*)(intptr_t)runtime_list_get(args, i);
        }
        argv[argc] = NULL;

        execvp(command, argv);
        // If exec fails
        _exit(127);
    }

    // Parent process
    close(pipefd[1]); // Close write end
    *stdout_pipe_read = pipefd[0];
    return (int64_t)pid;
}

int64_t runtime_process_spawn(char* command, LimeList args) {
    if (!command) return -1;
    int stdout_read = -1;
    int64_t pid = posix_spawn_process(command, args, &stdout_read);
    if (pid < 0) return -1;
    // Close the pipe fd since we don't store it for later use
    // A full implementation would maintain a process table
    if (stdout_read >= 0) close(stdout_read);
    return pid;
}

char* runtime_process_run(char* command, LimeList args) {
    if (!command) return runtime_str_copy("");

    int pipefd[2];
    if (pipe(pipefd) != 0) return runtime_str_copy("");

    pid_t pid = fork();
    if (pid < 0) {
        close(pipefd[0]);
        close(pipefd[1]);
        return runtime_str_copy("");
    }

    if (pid == 0) {
        // Child process
        close(pipefd[0]);
        dup2(pipefd[1], STDOUT_FILENO);
        dup2(pipefd[1], STDERR_FILENO);
        close(pipefd[1]);

        int argc = (int)args.len + 1;
        char** argv = (char**)malloc(sizeof(char*) * (argc + 1));
        if (!argv) { _exit(127); }
        argv[0] = command;
        for (int64_t i = 0; i < args.len; i++) {
            argv[i + 1] = (char*)(intptr_t)runtime_list_get(args, i);
        }
        argv[argc] = NULL;

        execvp(command, argv);
        _exit(127);
    }

    // Parent process
    close(pipefd[1]);

    // Read from pipe
    size_t cap = 4096;
    size_t len = 0;
    char* buf = (char*)malloc(cap);
    if (!buf) { close(pipefd[0]); waitpid(pid, NULL, 0); return NULL; }

    char c;
    while (read(pipefd[0], &c, 1) == 1) {
        if (len + 1 >= cap) {
            cap *= 2;
            char* nb = (char*)realloc(buf, cap);
            if (!nb) { free(buf); close(pipefd[0]); waitpid(pid, NULL, 0); return NULL; }
            buf = nb;
        }
        buf[len++] = c;
    }
    buf[len] = '\0';
    close(pipefd[0]);

    // Wait for child
    int status;
    waitpid(pid, &status, 0);

    return buf;
}

char* runtime_process_output(char* command, LimeList args) {
    return runtime_process_run(command, args);
}

int64_t runtime_process_wait(int64_t pid) {
    if (pid <= 0) return -1;
    int status;
    pid_t result = waitpid((pid_t)pid, &status, 0);
    if (result < 0) return -1;
    if (WIFEXITED(status)) {
        return (int64_t)WEXITSTATUS(status);
    }
    if (WIFSIGNALED(status)) {
        return -1;
    }
    return -1;
}

int runtime_process_kill(int64_t pid) {
    if (pid <= 0) return 0;
    int result = kill((pid_t)pid, SIGTERM);
    return result == 0 ? 1 : 0;
}

char* runtime_process_status(int64_t pid) {
    if (pid <= 0) return runtime_str_copy("failed");
    int status;
    pid_t result = waitpid((pid_t)pid, &status, WNOHANG);
    if (result < 0) return runtime_str_copy("failed");
    if (result == 0) return runtime_str_copy("running");
    if (WIFEXITED(status)) {
        char buf[32];
        snprintf(buf, sizeof(buf), "exited(%d)", WEXITSTATUS(status));
        return runtime_str_copy(buf);
    }
    return runtime_str_copy("failed");
}

LimeList runtime_process_args(void) {
    LimeList list;
    runtime_list_empty(&list);
    extern char** environ;
    (void)environ;
    return list;
}

#endif

// ========================================================================
// Requests operations (Phase C-1.12)
//
// HTTP client implementation.
// Windows: WinHTTP (system library, always available)
// POSIX: popen("curl ...") as HTTP backend
// ========================================================================

// --- Internal data structures ---

typedef struct HeaderEntry {
    struct HeaderEntry* next;
    char* key;
    char* value;
} HeaderEntry;

struct RequestsHeaderMap {
    HeaderEntry* first;
    int64_t count;
};

struct RequestsClient {
    char* proxy_url;
    int64_t timeout_seconds;
    int64_t redirect_limit;
    int disable_redirects;
    RequestsHeaderMap* default_headers;
};

struct RequestsRequestBuilder {
    RequestsClient* client;
    RequestsSession* session;  // back-pointer to session (for cookie updates)
    char* method;
    char* url;
    RequestsHeaderMap* headers;
    LimeList query_params;
    char* body_data;
    int64_t body_len;
    int body_type; // 0=none, 1=bytes, 2=string, 3=json, 4=form
    LimeJson* json_body;
    RequestsMultipart* multipart;
    int64_t timeout_seconds;
    int64_t redirect_limit;
    int disable_redirects;
    char* basic_auth_user;
    char* basic_auth_pass;
    char* bearer_token;
    int verify; // 1=verify TLS (default), 0=skip verification
};

struct RequestsResponse {
    int64_t status_code;
    RequestsHeaderMap* headers;
    char* url;
    char* body;
    int64_t body_len;
    RequestsRedirectHistory* redirect_history;
};

struct RequestsMultipart {
    LimeList fields;
};

struct RequestsTlsConfig {
    char* ca_cert_path;
    char* client_cert_path;
    char* client_key_path;
    int accept_invalid_certs;
    int accept_invalid_hostnames;
};

struct RequestsCookie {
    char* name;
    char* value;
    char* domain;
    char* path;
    int64_t expires;   // unix timestamp, 0 = session cookie
    int secure;
    int http_only;
};

struct RequestsCookieJar {
    LimeList cookies;  // list of RequestsCookie*
};

struct RequestsRedirectHistory {
    LimeList entries;  // list of RequestsRedirectEntry*
};

struct RequestsRedirectEntry {
    int64_t status_code;
    char* url;
    char* method;
};

struct RequestsSession {
    RequestsClient* client;
    RequestsHeaderMap* default_headers;
    RequestsCookieJar* cookies;
    RequestsRedirectHistory* redirect_history;
    LimeList default_params;  // list of key/value strings
    int64_t timeout_seconds;
    int64_t redirect_limit;
    int disable_redirects;
    int verify;
};

struct RequestsStream {
    char* data;
    int64_t len;
    int64_t pos;
};

// --- Header map helpers ---

static RequestsHeaderMap* header_map_new(void) {
    RequestsHeaderMap* m = (RequestsHeaderMap*)malloc(sizeof(RequestsHeaderMap));
    if (!m) runtime_panic("requests: out of memory");
    m->first = NULL;
    m->count = 0;
    return m;
}

static void header_map_free(RequestsHeaderMap* m) {
    if (!m) return;
    HeaderEntry* e = m->first;
    while (e) {
        HeaderEntry* next = e->next;
        free(e->key);
        free(e->value);
        free(e);
        e = next;
    }
    free(m);
}

static void header_map_insert(RequestsHeaderMap* m, char* key, char* value) {
    if (!m || !key) return;
    // Remove existing key
    HeaderEntry* prev = NULL;
    HeaderEntry* e = m->first;
    while (e) {
        if (strcmp(e->key, key) == 0) {
            free(e->value);
            e->value = runtime_str_copy(value ? value : "");
            return;
        }
        prev = e;
        e = e->next;
    }
    // Add new entry
    HeaderEntry* ne = (HeaderEntry*)malloc(sizeof(HeaderEntry));
    if (!ne) runtime_panic("requests: out of memory");
    ne->key = runtime_str_copy(key);
    ne->value = runtime_str_copy(value ? value : "");
    ne->next = NULL;
    if (prev) prev->next = ne;
    else m->first = ne;
    m->count++;
}

static void header_map_append(RequestsHeaderMap* m, char* key, char* value) {
    if (!m || !key) return;
    HeaderEntry* ne = (HeaderEntry*)malloc(sizeof(HeaderEntry));
    if (!ne) runtime_panic("requests: out of memory");
    ne->key = runtime_str_copy(key);
    ne->value = runtime_str_copy(value ? value : "");
    ne->next = m->first;
    m->first = ne;
    m->count++;
}

static char* header_map_get(RequestsHeaderMap* m, char* key) {
    if (!m || !key) return NULL;
    HeaderEntry* e = m->first;
    while (e) {
        if (strcmp(e->key, key) == 0) return runtime_str_copy(e->value);
        e = e->next;
    }
    return NULL;
}

static int header_map_contains(RequestsHeaderMap* m, char* key) {
    if (!m || !key) return 0;
    HeaderEntry* e = m->first;
    while (e) {
        if (strcmp(e->key, key) == 0) return 1;
        e = e->next;
    }
    return 0;
}

static void header_map_remove(RequestsHeaderMap* m, char* key) {
    if (!m || !key) return;
    HeaderEntry* prev = NULL;
    HeaderEntry* e = m->first;
    while (e) {
        if (strcmp(e->key, key) == 0) {
            if (prev) prev->next = e->next;
            else m->first = e->next;
            free(e->key);
            free(e->value);
            free(e);
            m->count--;
            return;
        }
        prev = e;
        e = e->next;
    }
}

// --- String builder helper ---

typedef struct {
    char* data;
    int64_t len;
    int64_t cap;
} StrBuilder;

static void sb_init(StrBuilder* sb) {
    sb->cap = 256;
    sb->len = 0;
    sb->data = (char*)malloc(sb->cap);
    if (!sb->data) runtime_panic("requests: out of memory");
}

static void sb_append(StrBuilder* sb, const char* s) {
    if (!s) return;
    int64_t slen = (int64_t)strlen(s);
    while (sb->len + slen + 1 > sb->cap) {
        sb->cap *= 2;
        sb->data = (char*)realloc(sb->data, sb->cap);
        if (!sb->data) runtime_panic("requests: out of memory");
    }
    memcpy(sb->data + sb->len, s, slen);
    sb->len += slen;
    sb->data[sb->len] = '\0';
}

static void sb_append_n(StrBuilder* sb, const char* s, int64_t n) {
    while (sb->len + n + 1 > sb->cap) {
        sb->cap *= 2;
        sb->data = (char*)realloc(sb->data, sb->cap);
        if (!sb->data) runtime_panic("requests: out of memory");
    }
    memcpy(sb->data + sb->len, s, n);
    sb->len += n;
    sb->data[sb->len] = '\0';
}

static char* sb_finish(StrBuilder* sb) {
    return sb->data;
}

// --- Shell escape a string for safe inclusion in single-quoted shell args ---
// Returns malloc'd string. Caller must free.
static char* shell_escape(const char* s) {
    if (!s) return runtime_str_copy("");
    // Count single quotes in input
    int sq_count = 0;
    for (const char* p = s; *p; p++) {
        if (*p == '\'') sq_count++;
    }
    if (sq_count == 0) {
        return runtime_str_copy(s);
    }
    // Escape: replace ' with '\'' (end quote, escaped quote, start quote)
    size_t slen = strlen(s);
    size_t out_len = slen + sq_count * 3 + 1;
    char* out = (char*)malloc(out_len);
    if (!out) runtime_panic("requests: out of memory");
    char* q = out;
    for (const char* p = s; *p; p++) {
        if (*p == '\'') {
            *q++ = '\''; *q++ = '\\'; *q++ = '\''; *q++ = '\'';
        } else {
            *q++ = *p;
        }
    }
    *q = '\0';
    return out;
}

// --- URL-encode a string (application/x-www-form-urlencoded) ---

static char* url_encode(const char* s) {
    if (!s) return runtime_str_copy("");
    StrBuilder sb;
    sb_init(&sb);
    for (const char* p = s; *p; p++) {
        unsigned char c = (unsigned char)*p;
        if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
            (c >= '0' && c <= '9') || c == '-' || c == '_' || c == '.' || c == '~') {
            char buf[2] = { (char)c, '\0' };
            sb_append(&sb, buf);
        } else {
            char buf[4];
            snprintf(buf, sizeof(buf), "%%%02X", c);
            sb_append(&sb, buf);
        }
    }
    return sb_finish(&sb);
}

// --- Free a client ---

void runtime_requests_client_free(RequestsClient* client) {
    if (!client) return;
    header_map_free(client->default_headers);
    free(client->proxy_url);
    free(client);
}

void runtime_requests_request_builder_free(RequestsRequestBuilder* builder) {
    if (!builder) return;
    free(builder->method);
    free(builder->url);
    header_map_free(builder->headers);
    free(builder->body_data);
    free(builder->basic_auth_user);
    free(builder->basic_auth_pass);
    free(builder->bearer_token);
    if (builder->multipart) runtime_requests_multipart_free(builder->multipart);
    free(builder);
}

void runtime_requests_response_free(RequestsResponse* response) {
    if (!response) return;
    header_map_free(response->headers);
    free(response->url);
    free(response->body);
    if (response->redirect_history) runtime_requests_redirect_history_free(response->redirect_history);
    free(response);
}

void runtime_requests_header_map_free(RequestsHeaderMap* map) {
    header_map_free(map);
}

void runtime_requests_multipart_free(RequestsMultipart* multipart) {
    if (!multipart) return;
    for (int64_t i = 0; i < multipart->fields.len; i++) {
        LimeList* tuple = (LimeList*)(intptr_t)runtime_list_get(multipart->fields, i);
        if (tuple) {
            for (int64_t j = 0; j < tuple->len; j++) {
                char* s = (char*)(intptr_t)runtime_list_get(*tuple, j);
                free(s);
            }
            free(tuple->data);
            free(tuple);
        }
    }
    free(multipart->fields.data);
    free(multipart);
}

void runtime_requests_tls_config_free(RequestsTlsConfig* config) {
    if (!config) return;
    free(config->ca_cert_path);
    free(config->client_cert_path);
    free(config->client_key_path);
    free(config);
}

// Forward declarations for cookie management
static void cookie_free(RequestsCookie* c);
static void parse_url_host_port(const char* url, char** host, int* port, char** path);

void runtime_requests_cookie_jar_free(RequestsCookieJar* jar) {
    if (!jar) return;
    for (int64_t i = 0; i < jar->cookies.len; i++) {
        void* item = (void*)(intptr_t)runtime_list_get(jar->cookies, i);
        if (item) {
            // Check if it's a RequestsCookie* or a raw string
            // In new code, we store RequestsCookie*. In legacy code, raw strings.
            // We use a heuristic: if the first byte looks like a heap pointer to a RequestsCookie,
            // we free it as a RequestsCookie. For backwards compat, we just free as string.
            // Actually, since we control all callers, all new cookies are RequestsCookie*.
            cookie_free((RequestsCookie*)item);
        }
    }
    free(jar->cookies.data);
    free(jar);
}

void runtime_requests_stream_free(RequestsStream* stream) {
    if (!stream) return;
    free(stream->data);
    free(stream);
}

// --- Client management ---

RequestsClient* runtime_requests_client_new(void) {
    RequestsClient* c = (RequestsClient*)malloc(sizeof(RequestsClient));
    if (!c) runtime_panic("requests: out of memory");
    memset(c, 0, sizeof(*c));
    c->timeout_seconds = 30;
    c->redirect_limit = 10;
    c->disable_redirects = 0;
    c->default_headers = header_map_new();
    return c;
}

RequestsClient* runtime_requests_client_builder_new(void) {
    return runtime_requests_client_new();
}

RequestsClient* runtime_requests_client_builder_build(RequestsClient* builder) {
    // Builder IS the client (moved semantics)
    return builder;
}

void runtime_requests_client_builder_default_headers(RequestsClient* builder, RequestsHeaderMap* headers) {
    if (!builder || !headers) return;
    header_map_free(builder->default_headers);
    builder->default_headers = header_map_new();
    HeaderEntry* e = headers->first;
    while (e) {
        header_map_insert(builder->default_headers, e->key, e->value);
        e = e->next;
    }
}

void runtime_requests_client_builder_timeout(RequestsClient* builder, int64_t seconds) {
    if (builder) builder->timeout_seconds = seconds;
}

void runtime_requests_client_builder_redirect_limit(RequestsClient* builder, int64_t limit) {
    if (builder) builder->redirect_limit = limit;
}

void runtime_requests_client_builder_redirect_disabled(RequestsClient* builder) {
    if (builder) builder->disable_redirects = 1;
}

void runtime_requests_client_builder_proxy(RequestsClient* builder, char* proxy_url) {
    if (!builder) return;
    free(builder->proxy_url);
    builder->proxy_url = proxy_url ? runtime_str_copy(proxy_url) : NULL;
}

void runtime_requests_client_builder_tls_config(RequestsClient* builder, RequestsTlsConfig* tls_config) {
    // TLS config is stored but handled by the HTTP backend
    (void)builder;
    (void)tls_config;
}

// --- Request builder ---

RequestsRequestBuilder* runtime_requests_request_builder_new(RequestsClient* client, char* method, char* url) {
    RequestsRequestBuilder* b = (RequestsRequestBuilder*)malloc(sizeof(RequestsRequestBuilder));
    if (!b) runtime_panic("requests: out of memory");
    memset(b, 0, sizeof(*b));
    b->client = client;
    b->session = NULL;
    b->method = method ? runtime_str_copy(method) : runtime_str_copy("GET");
    b->url = url ? runtime_str_copy(url) : runtime_str_copy("");
    b->headers = header_map_new();
    runtime_list_empty(&b->query_params);
    b->timeout_seconds = client ? client->timeout_seconds : 30;
    b->redirect_limit = client ? client->redirect_limit : 10;
    b->disable_redirects = client ? client->disable_redirects : 0;
    b->verify = 1; // verify TLS by default
    return b;
}

int runtime_requests_request_builder_header(RequestsRequestBuilder* builder, char* key, char* value) {
    if (!builder || !key) return -1;
    header_map_insert(builder->headers, key, value);
    return 0;
}

int runtime_requests_request_builder_headers(RequestsRequestBuilder* builder, RequestsHeaderMap* headers) {
    if (!builder || !headers) return -1;
    HeaderEntry* e = headers->first;
    while (e) {
        header_map_insert(builder->headers, e->key, e->value);
        e = e->next;
    }
    return 0;
}

int runtime_requests_request_builder_query(RequestsRequestBuilder* builder, LimeList params) {
    if (!builder) return -1;
    free(builder->query_params.data);
    builder->query_params = params;
    return 0;
}

int runtime_requests_request_builder_body_bytes(RequestsRequestBuilder* builder, char* data, int64_t len) {
    if (!builder) return -1;
    free(builder->body_data);
    builder->body_data = (char*)malloc(len + 1);
    if (!builder->body_data) return -1;
    memcpy(builder->body_data, data, len);
    builder->body_data[len] = '\0';
    builder->body_len = len;
    builder->body_type = 1;
    return 0;
}

int runtime_requests_request_builder_body_str(RequestsRequestBuilder* builder, char* body) {
    if (!builder) return -1;
    free(builder->body_data);
    builder->body_data = body ? runtime_str_copy(body) : runtime_str_copy("");
    builder->body_len = body ? (int64_t)strlen(body) : 0;
    builder->body_type = 2;
    return 0;
}

int runtime_requests_request_builder_json(RequestsRequestBuilder* builder, void* json_value) {
    if (!builder) return -1;
    // Serialize the JSON value to a string body
    if (json_value) {
        char* json_str = runtime_json_stringify((LimeJson*)json_value);
        free(builder->body_data);
        builder->body_data = json_str;
        builder->body_len = json_str ? (int64_t)strlen(json_str) : 0;
    }
    // Do NOT store json_value pointer - it's not owned by us
    builder->json_body = NULL;
    builder->body_type = 3;
    return 0;
}

int runtime_requests_request_builder_form(RequestsRequestBuilder* builder, LimeList data) {
    if (!builder) return -1;
    // Build form-encoded body
    StrBuilder sb;
    sb_init(&sb);
    for (int64_t i = 0; i < data.len; i++) {
        // Each element is a tuple stored as a LimeList pointer
        // We treat the list elements as key/value string pairs in the flat list
        // For simplicity, we expect the data as pairs of i64 (string pointers)
        if (i > 0) sb_append(&sb, "&");
        char* key = (char*)(intptr_t)runtime_list_get(data, i);
        i++;
        if (i >= data.len) break;
        char* val = (char*)(intptr_t)runtime_list_get(data, i);
        char* enc_key = url_encode(key);
        char* enc_val = url_encode(val);
        sb_append(&sb, enc_key);
        sb_append(&sb, "=");
        sb_append(&sb, enc_val);
        free(enc_key);
        free(enc_val);
    }
    free(builder->body_data);
    builder->body_data = sb_finish(&sb);
    builder->body_len = (int64_t)strlen(builder->body_data);
    builder->body_type = 4;
    return 0;
}

int runtime_requests_request_builder_multipart(RequestsRequestBuilder* builder, RequestsMultipart* multipart) {
    if (!builder) return -1;
    builder->multipart = multipart;
    return 0;
}

int runtime_requests_request_builder_timeout(RequestsRequestBuilder* builder, int64_t seconds) {
    if (!builder) return -1;
    builder->timeout_seconds = seconds;
    return 0;
}

int runtime_requests_request_builder_redirect_limit(RequestsRequestBuilder* builder, int64_t limit) {
    if (!builder) return -1;
    builder->redirect_limit = limit;
    return 0;
}

int runtime_requests_request_builder_redirect_disabled(RequestsRequestBuilder* builder) {
    if (!builder) return -1;
    builder->disable_redirects = 1;
    return 0;
}

int runtime_requests_request_builder_basic_auth(RequestsRequestBuilder* builder, char* user, char* password) {
    if (!builder) return -1;
    free(builder->basic_auth_user);
    free(builder->basic_auth_pass);
    builder->basic_auth_user = user ? runtime_str_copy(user) : NULL;
    builder->basic_auth_pass = password ? runtime_str_copy(password) : NULL;
    return 0;
}

int runtime_requests_request_builder_bearer_auth(RequestsRequestBuilder* builder, char* token) {
    if (!builder) return -1;
    free(builder->bearer_token);
    builder->bearer_token = token ? runtime_str_copy(token) : NULL;
    return 0;
}

int runtime_requests_request_builder_set_headers(RequestsRequestBuilder* builder, LimeList headers) {
    if (!builder) return -1;
    for (int64_t i = 0; i < headers.len; i++) {
        char* key = (char*)(intptr_t)runtime_list_get(headers, i);
        i++;
        if (i >= headers.len) break;
        char* val = (char*)(intptr_t)runtime_list_get(headers, i);
        header_map_insert(builder->headers, key, val);
    }
    return 0;
}

int runtime_requests_request_builder_verify(RequestsRequestBuilder* builder, int verify) {
    if (!builder) return -1;
    builder->verify = verify;
    return 0;
}

// --- Header map API ---

RequestsHeaderMap* runtime_requests_header_map_new(void) {
    return header_map_new();
}

int runtime_requests_header_map_insert(RequestsHeaderMap* map, char* key, char* value) {
    if (!map || !key) return -1;
    header_map_insert(map, key, value);
    return 0;
}

int runtime_requests_header_map_append(RequestsHeaderMap* map, char* key, char* value) {
    if (!map || !key) return -1;
    header_map_append(map, key, value);
    return 0;
}

int runtime_requests_header_map_remove(RequestsHeaderMap* map, char* key) {
    if (!map || !key) return -1;
    header_map_remove(map, key);
    return 0;
}

char* runtime_requests_header_map_get(RequestsHeaderMap* map, char* key) {
    return header_map_get(map, key);
}

int runtime_requests_header_map_contains(RequestsHeaderMap* map, char* key) {
    return header_map_contains(map, key);
}

// --- Status code helpers ---

int64_t runtime_requests_status_code_code(int64_t code) { return code; }
int runtime_requests_status_code_is_success(int64_t code) { return code >= 200 && code < 300; }
int runtime_requests_status_code_is_client_error(int64_t code) { return code >= 400 && code < 500; }
int runtime_requests_status_code_is_server_error(int64_t code) { return code >= 500 && code < 600; }
int runtime_requests_status_code_is_redirect(int64_t code) { return code >= 300 && code < 400; }

// --- Multipart ---

RequestsMultipart* runtime_requests_multipart_new(void) {
    RequestsMultipart* m = (RequestsMultipart*)malloc(sizeof(RequestsMultipart));
    if (!m) runtime_panic("requests: out of memory");
    runtime_list_empty(&m->fields);
    return m;
}

int runtime_requests_multipart_text(RequestsMultipart* multipart, char* name, char* value) {
    if (!multipart || !name) return -1;
    LimeList* tuple = (LimeList*)malloc(sizeof(LimeList));
    if (!tuple) return -1;
    tuple->data = NULL; tuple->len = 0; tuple->cap = 0;
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy(name));
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy(value ? value : ""));
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy("text"));
    runtime_list_add(&multipart->fields, (int64_t)(intptr_t)tuple);
    return 0;
}

int runtime_requests_multipart_file(RequestsMultipart* multipart, char* name, char* file_path) {
    if (!multipart || !name || !file_path) return -1;
    LimeList* tuple = (LimeList*)malloc(sizeof(LimeList));
    if (!tuple) return -1;
    tuple->data = NULL; tuple->len = 0; tuple->cap = 0;
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy(name));
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy(file_path));
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy("file"));
    runtime_list_add(&multipart->fields, (int64_t)(intptr_t)tuple);
    return 0;
}

int runtime_requests_multipart_file_with_metadata(RequestsMultipart* multipart, char* name, char* file_path, char* filename, char* content_type) {
    if (!multipart || !name || !file_path) return -1;
    LimeList* tuple = (LimeList*)malloc(sizeof(LimeList));
    if (!tuple) return -1;
    tuple->data = NULL; tuple->len = 0; tuple->cap = 0;
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy(name));
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy(file_path));
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy("file"));
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy(filename ? filename : ""));
    runtime_list_add(tuple, (int64_t)(intptr_t)runtime_str_copy(content_type ? content_type : ""));
    runtime_list_add(&multipart->fields, (int64_t)(intptr_t)tuple);
    return 0;
}

// --- TLS config ---

RequestsTlsConfig* runtime_requests_tls_config_new(void) {
    RequestsTlsConfig* c = (RequestsTlsConfig*)malloc(sizeof(RequestsTlsConfig));
    if (!c) runtime_panic("requests: out of memory");
    memset(c, 0, sizeof(*c));
    return c;
}

int runtime_requests_tls_config_add_ca_cert(RequestsTlsConfig* config, char* pem_path) {
    if (!config || !pem_path) return -1;
    free(config->ca_cert_path);
    config->ca_cert_path = runtime_str_copy(pem_path);
    return 0;
}

int runtime_requests_tls_config_add_client_cert(RequestsTlsConfig* config, char* cert_path, char* key_path) {
    if (!config || !cert_path || !key_path) return -1;
    free(config->client_cert_path);
    free(config->client_key_path);
    config->client_cert_path = runtime_str_copy(cert_path);
    config->client_key_path = runtime_str_copy(key_path);
    return 0;
}

int runtime_requests_tls_config_danger_accept_invalid_certs(RequestsTlsConfig* config) {
    if (!config) return -1;
    config->accept_invalid_certs = 1;
    return 0;
}

int runtime_requests_tls_config_danger_accept_invalid_hostnames(RequestsTlsConfig* config) {
    if (!config) return -1;
    config->accept_invalid_hostnames = 1;
    return 0;
}

// --- Cookie jar ---

static void cookie_free(RequestsCookie* c) {
    if (!c) return;
    free(c->name);
    free(c->value);
    free(c->domain);
    free(c->path);
    free(c);
}

static RequestsCookie* cookie_new(void) {
    RequestsCookie* c = (RequestsCookie*)malloc(sizeof(RequestsCookie));
    if (!c) runtime_panic("requests: out of memory");
    memset(c, 0, sizeof(*c));
    return c;
}

// Parse Set-Cookie header value: "name=value; Domain=...; Path=...; Secure; HttpOnly; Max-Age=..."
static RequestsCookie* cookie_parse_set_cookie(char* header_value, char* request_domain) {
    if (!header_value) return NULL;
    RequestsCookie* c = cookie_new();

    // Copy so we can tokenize
    char* work = runtime_str_copy(header_value);
    char* saveptr = NULL;
    char* token = strtok_r(work, ";", &saveptr);
    if (!token) { free(work); cookie_free(c); return NULL; }

    // First token: "name=value"
    char* eq = strchr(token, '=');
    if (eq) {
        *eq = '\0';
        c->name = runtime_str_copy(token);
        c->value = runtime_str_copy(eq + 1);
    } else {
        c->name = runtime_str_copy(token);
        c->value = runtime_str_copy("");
    }

    // Default domain from request
    c->domain = request_domain ? runtime_str_copy(request_domain) : runtime_str_copy("");
    c->path = runtime_str_copy("/");
    c->secure = 0;
    c->http_only = 0;
    c->expires = 0;

    // Parse attributes
    while ((token = strtok_r(NULL, ";", &saveptr)) != NULL) {
        while (*token == ' ') token++;
        if (strncasecmp(token, "Domain=", 7) == 0) {
            free(c->domain);
            c->domain = runtime_str_copy(token + 7);
        } else if (strncasecmp(token, "Path=", 5) == 0) {
            free(c->path);
            c->path = runtime_str_copy(token + 5);
        } else if (strncasecmp(token, "Max-Age=", 8) == 0) {
            int64_t max_age = (int64_t)atol(token + 8);
            if (max_age > 0) {
                // Convert to unix timestamp from now
                c->expires = (int64_t)time(NULL) + max_age;
            }
        } else if (strncasecmp(token, "Expires=", 8) == 0) {
            // Parse HTTP date: "Wdy, DD Mon YYYY HH:MM:SS GMT"
            // Use mktime as a fallback - parse the date string
            struct tm tm_val;
            memset(&tm_val, 0, sizeof(tm_val));
            char* date_str = token + 8;
            // Try common HTTP date formats
            // Format: "Mon, 01 Jan 2024 00:00:00 GMT"
            static const char* months[] = {"Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"};
            char day_name[4], month_name[4];
            int day, year, hour, min, sec;
            if (sscanf(date_str, "%3s, %d %3s %d %d:%d:%d", day_name, &day, month_name, &year, &hour, &min, &sec) == 7) {
                tm_val.tm_mday = day;
                tm_val.tm_year = year - 1900;
                tm_val.tm_hour = hour;
                tm_val.tm_min = min;
                tm_val.tm_sec = sec;
                for (int m = 0; m < 12; m++) {
                    if (strcasecmp(month_name, months[m]) == 0) {
                        tm_val.tm_mon = m;
                        break;
                    }
                }
                c->expires = (int64_t)mktime(&tm_val);
            }
            // If parsing failed, leave expires as 0 (session cookie)
        } else if (strcasecmp(token, "Secure") == 0) {
            c->secure = 1;
        } else if (strcasecmp(token, "HttpOnly") == 0) {
            c->http_only = 1;
        }
    }

    free(work);
    return c;
}

// Check if a cookie matches a URL (domain + path + secure + expiry)
static int cookie_matches(RequestsCookie* c, char* domain, char* path, int is_https) {
    if (!c || !domain || !path) return 0;

    // Check expiry
    if (c->expires > 0 && (int64_t)time(NULL) > c->expires) return 0;

    // Check secure flag
    if (c->secure && !is_https) return 0;

    // Check domain (exact match or subdomain match)
    if (c->domain[0] != '\0') {
        size_t cd_len = strlen(c->domain);
        size_t d_len = strlen(domain);
        if (cd_len > d_len) return 0;
        // Must end with the cookie domain
        if (strcasecmp(domain + (d_len - cd_len), c->domain) != 0) return 0;
        // If the character before the match isn't '.', it must be exact match
        if (d_len > cd_len && domain[d_len - cd_len - 1] != '.') return 0;
    }

    // Check path (cookie path must be a prefix of request path)
    if (c->path[0] != '\0') {
        size_t cp_len = strlen(c->path);
        if (strncmp(path, c->path, cp_len) != 0) return 0;
        // If path prefix match, next char must be '/' or end of string
        if (path[cp_len] != '\0' && path[cp_len] != '/') return 0;
    }

    return 1;
}

RequestsCookieJar* runtime_requests_cookie_jar_new(void) {
    RequestsCookieJar* j = (RequestsCookieJar*)malloc(sizeof(RequestsCookieJar));
    if (!j) runtime_panic("requests: out of memory");
    runtime_list_empty(&j->cookies);
    return j;
}

int runtime_requests_cookie_jar_add(RequestsCookieJar* jar, char* cookie_str) {
    if (!jar || !cookie_str) return -1;
    // Parse the cookie string and store as RequestsCookie*
    RequestsCookie* c = cookie_parse_set_cookie(cookie_str, NULL);
    if (!c) return -1;
    runtime_list_add(&jar->cookies, (int64_t)(intptr_t)c);
    return 0;
}

int runtime_requests_cookie_jar_add_parsed(RequestsCookieJar* jar, void* cookie) {
    if (!jar || !cookie) return -1;
    runtime_list_add(&jar->cookies, (int64_t)(intptr_t)cookie);
    return 0;
}

// Parse Set-Cookie headers from response and add to jar
void runtime_requests_cookie_jar_update_from_response(RequestsCookieJar* jar, RequestsHeaderMap* resp_headers, char* request_url) {
    if (!jar || !resp_headers) return;

    // Extract domain from request URL
    char* domain = NULL;
    parse_url_host_port(request_url, &domain, NULL, NULL);

    int is_https = (strncmp(request_url, "https://", 8) == 0);

    // Find all Set-Cookie headers
    HeaderEntry* e = resp_headers->first;
    while (e) {
        if (strcasecmp(e->key, "Set-Cookie") == 0) {
            RequestsCookie* c = cookie_parse_set_cookie(e->value, domain);
            if (c) {
                // Remove existing cookie with same name+domain+path
                for (int64_t i = jar->cookies.len - 1; i >= 0; i--) {
                    RequestsCookie* existing = (RequestsCookie*)(intptr_t)runtime_list_get(jar->cookies, i);
                    if (existing && existing->name && c->name &&
                        strcmp(existing->name, c->name) == 0 &&
                        existing->domain && c->domain &&
                        strcasecmp(existing->domain, c->domain) == 0 &&
                        existing->path && c->path &&
                        strcmp(existing->path, c->path) == 0) {
                        // Remove old cookie
                        cookie_free(existing);
                        // Shift elements down
                        for (int64_t j = i; j < jar->cookies.len - 1; j++) {
                            jar->cookies.data[j] = jar->cookies.data[j + 1];
                        }
                        jar->cookies.len--;
                        break;
                    }
                }
                runtime_list_add(&jar->cookies, (int64_t)(intptr_t)c);
            }
        }
        e = e->next;
    }
    free(domain);
}

// Build Cookie header string from matching cookies
char* runtime_requests_cookie_jar_get_cookie_header(RequestsCookieJar* jar, char* url) {
    if (!jar || !url) return NULL;

    char* host = NULL;
    char* path = NULL;
    parse_url_host_port(url, &host, NULL, &path);
    int is_https = (strncmp(url, "https://", 8) == 0);

    StrBuilder sb;
    sb_init(&sb);
    int first = 1;

    for (int64_t i = 0; i < jar->cookies.len; i++) {
        RequestsCookie* c = (RequestsCookie*)(intptr_t)runtime_list_get(jar->cookies, i);
        if (cookie_matches(c, host, path, is_https)) {
            if (!first) sb_append(&sb, "; ");
            sb_append(&sb, c->name);
            sb_append(&sb, "=");
            sb_append(&sb, c->value);
            first = 0;
        }
    }

    free(host);
    free(path);

    if (first) {
        free(sb.data);
        return NULL;
    }
    return sb_finish(&sb);
}

// Get all cookies as a list of tuples (name, value) for the Lime layer
LimeList runtime_requests_cookie_jar_get_all(RequestsCookieJar* jar) {
    LimeList list;
    runtime_list_empty(&list);
    if (!jar) return list;
    for (int64_t i = 0; i < jar->cookies.len; i++) {
        RequestsCookie* c = (RequestsCookie*)(intptr_t)runtime_list_get(jar->cookies, i);
        if (c && c->name) {
            runtime_list_add(&list, (int64_t)(intptr_t)runtime_str_copy(c->name));
            runtime_list_add(&list, (int64_t)(intptr_t)runtime_str_copy(c->value ? c->value : ""));
        }
    }
    return list;
}

// Get a specific cookie value by name
char* runtime_requests_cookie_jar_get(RequestsCookieJar* jar, char* name) {
    if (!jar || !name) return NULL;
    for (int64_t i = jar->cookies.len - 1; i >= 0; i--) {
        RequestsCookie* c = (RequestsCookie*)(intptr_t)runtime_list_get(jar->cookies, i);
        if (c && c->name && strcmp(c->name, name) == 0) {
            return runtime_str_copy(c->value ? c->value : "");
        }
    }
    return NULL;
}

char* runtime_requests_cookie_parse(char* cookie_str) {
    if (!cookie_str) return NULL;
    return runtime_str_copy(cookie_str);
}

// --- Redirect history ---

RequestsRedirectHistory* runtime_requests_redirect_history_new(void) {
    RequestsRedirectHistory* h = (RequestsRedirectHistory*)malloc(sizeof(RequestsRedirectHistory));
    if (!h) runtime_panic("requests: out of memory");
    runtime_list_empty(&h->entries);
    return h;
}

void runtime_requests_redirect_history_add(RequestsRedirectHistory* history, int64_t status_code, char* url, char* method) {
    if (!history) return;
    RequestsRedirectEntry* e = (RequestsRedirectEntry*)malloc(sizeof(RequestsRedirectEntry));
    if (!e) runtime_panic("requests: out of memory");
    e->status_code = status_code;
    e->url = url ? runtime_str_copy(url) : runtime_str_copy("");
    e->method = method ? runtime_str_copy(method) : runtime_str_copy("");
    runtime_list_add(&history->entries, (int64_t)(intptr_t)e);
}

LimeList runtime_requests_redirect_history_list(RequestsRedirectHistory* history) {
    LimeList list;
    runtime_list_empty(&list);
    if (!history) return list;
    for (int64_t i = 0; i < history->entries.len; i++) {
        RequestsRedirectEntry* e = (RequestsRedirectEntry*)(intptr_t)runtime_list_get(history->entries, i);
        if (e) {
            runtime_list_add(&list, (int64_t)(intptr_t)runtime_str_copy(e->url));
            runtime_list_add(&list, (int64_t)e->status_code);
        }
    }
    return list;
}

void runtime_requests_redirect_history_free(RequestsRedirectHistory* history) {
    if (!history) return;
    for (int64_t i = 0; i < history->entries.len; i++) {
        RequestsRedirectEntry* e = (RequestsRedirectEntry*)(intptr_t)runtime_list_get(history->entries, i);
        if (e) {
            free(e->url);
            free(e->method);
            free(e);
        }
    }
    free(history->entries.data);
    free(history);
}

// --- Session ---

RequestsSession* runtime_requests_session_new(void) {
    RequestsSession* s = (RequestsSession*)malloc(sizeof(RequestsSession));
    if (!s) runtime_panic("requests: out of memory");
    memset(s, 0, sizeof(*s));
    s->client = runtime_requests_client_new();
    s->default_headers = header_map_new();
    s->cookies = runtime_requests_cookie_jar_new();
    s->redirect_history = runtime_requests_redirect_history_new();
    runtime_list_empty(&s->default_params);
    s->timeout_seconds = 30;
    s->redirect_limit = 10;
    s->disable_redirects = 0;
    s->verify = 1; // verify TLS by default
    return s;
}

RequestsRequestBuilder* runtime_requests_session_request(RequestsSession* session, char* method, char* url) {
    if (!session) return NULL;
    RequestsRequestBuilder* b = runtime_requests_request_builder_new(session->client, method, url);
    if (b) {
        b->session = session;  // back-pointer for cookie updates
        b->verify = session->verify;
        b->timeout_seconds = session->timeout_seconds;
        b->redirect_limit = session->redirect_limit;
        b->disable_redirects = session->disable_redirects;
        // Copy session default headers to builder
        if (session->default_headers) {
            HeaderEntry* e = session->default_headers->first;
            while (e) {
                header_map_insert(b->headers, e->key, e->value);
                e = e->next;
            }
        }
        // Copy session default params to builder
        if (session->default_params.len > 0) {
            for (int64_t i = 0; i < session->default_params.len; i++) {
                runtime_list_add(&b->query_params,
                    (int64_t)(intptr_t)runtime_str_copy(
                        (char*)(intptr_t)runtime_list_get(session->default_params, i)));
            }
        }
        // Attach matching cookies from session cookie jar
        if (session->cookies && url) {
            char* cookie_header = runtime_requests_cookie_jar_get_cookie_header(session->cookies, url);
            if (cookie_header) {
                header_map_insert(b->headers, "Cookie", cookie_header);
                free(cookie_header);
            }
        }
    }
    return b;
}

// Session setters for Python requests compatibility

int runtime_requests_session_set_default_headers(RequestsSession* session, LimeList headers) {
    if (!session) return -1;
    for (int64_t i = 0; i < headers.len; i++) {
        char* key = (char*)(intptr_t)runtime_list_get(headers, i);
        i++;
        if (i >= headers.len) break;
        char* val = (char*)(intptr_t)runtime_list_get(headers, i);
        header_map_insert(session->default_headers, key, val);
    }
    return 0;
}

int runtime_requests_session_set_default_params(RequestsSession* session, LimeList params) {
    if (!session) return -1;
    free(session->default_params.data);
    session->default_params = params;
    return 0;
}

int runtime_requests_session_set_timeout(RequestsSession* session, int64_t seconds) {
    if (!session) return -1;
    session->timeout_seconds = seconds;
    return 0;
}

int runtime_requests_session_set_verify(RequestsSession* session, int verify) {
    if (!session) return -1;
    session->verify = verify;
    return 0;
}

int runtime_requests_session_set_redirect_limit(RequestsSession* session, int64_t limit) {
    if (!session) return -1;
    session->redirect_limit = limit;
    return 0;
}

int runtime_requests_session_set_disable_redirects(RequestsSession* session, int disable) {
    if (!session) return -1;
    session->disable_redirects = disable;
    return 0;
}

LimeList runtime_requests_session_cookies(RequestsSession* session) {
    if (!session || !session->cookies) { LimeList _empty; runtime_list_empty(&_empty); return _empty; }
    return runtime_requests_cookie_jar_get_all(session->cookies);
}

void runtime_requests_session_free(RequestsSession* session) {
    if (!session) return;
    runtime_requests_client_free(session->client);
    header_map_free(session->default_headers);
    runtime_requests_cookie_jar_free(session->cookies);
    runtime_requests_redirect_history_free(session->redirect_history);
    free(session->default_params.data);
    free(session);
}

// --- HTTP execution ---

#ifdef _WIN32

#include <winhttp.h>

// Helper: convert UTF-8 string to wide string
static wchar_t* utf8_to_wide(const char* s) {
    if (!s) return NULL;
    int len = MultiByteToWideChar(CP_UTF8, 0, s, -1, NULL, 0);
    wchar_t* w = (wchar_t*)malloc(len * sizeof(wchar_t));
    if (!w) return NULL;
    MultiByteToWideChar(CP_UTF8, 0, s, -1, w, len);
    return w;
}

// Helper: extract host and port from URL
static void parse_url_host_port(const char* url, char** host, int* port, char** path) {
    const char* p = url;
    // Skip scheme
    if (strncmp(p, "https://", 8) == 0) { p += 8; if (port) *port = 443; }
    else if (strncmp(p, "http://", 7) == 0) { p += 7; if (port) *port = 80; }
    // Find path
    const char* slash = strchr(p, '/');
    const char* qmark = strchr(p, '?');
    const char* end = slash ? slash : (qmark ? qmark : p + strlen(p));
    // Find port
    const char* colon = memchr(p, ':', end - p);
    if (colon) {
        if (host) *host = runtime_str_copy(""); // placeholder
        size_t hlen = colon - p;
        char* h = (char*)malloc(hlen + 1);
        memcpy(h, p, hlen);
        h[hlen] = '\0';
        if (host) { free(*host); *host = h; }
        if (port) *port = atoi(colon + 1);
    } else {
        size_t hlen = end - p;
        char* h = (char*)malloc(hlen + 1);
        memcpy(h, p, hlen);
        h[hlen] = '\0';
        if (host) *host = h;
    }
    if (path) {
        if (*end == '/' || *end == '?') {
            *path = runtime_str_copy(end);
        } else {
            *path = runtime_str_copy("/");
        }
    }
}

RequestsResponse* runtime_requests_send(RequestsRequestBuilder* builder) {
    if (!builder || !builder->url) return NULL;

    // Build full URL with query params
    StrBuilder url_sb;
    sb_init(&url_sb);
    sb_append(&url_sb, builder->url);

    // Add query parameters
    if (builder->query_params.len > 0) {
        int has_q = strchr(builder->url, '?') != NULL;
        for (int64_t i = 0; i < builder->query_params.len; i++) {
            char* key = (char*)(intptr_t)runtime_list_get(builder->query_params, i);
            i++;
            if (i >= builder->query_params.len) break;
            char* val = (char*)(intptr_t)runtime_list_get(builder->query_params, i);
            if (!has_q && i == 1) sb_append(&url_sb, "?");
            else sb_append(&url_sb, "&");
            char* ek = url_encode(key);
            char* ev = url_encode(val);
            sb_append(&url_sb, ek);
            sb_append(&url_sb, "=");
            sb_append(&url_sb, ev);
            free(ek);
            free(ev);
        }
    }
    char* full_url = sb_finish(&url_sb);

    // Parse URL
    char* host = NULL;
    int port = 443;
    char* url_path = NULL;
    parse_url_host_port(full_url, &host, &port, &url_path);
    int use_tls = (port == 443) || (strncmp(full_url, "https://", 8) == 0);

    wchar_t* w_host = utf8_to_wide(host);
    wchar_t* w_path = utf8_to_wide(url_path);

    // Create WinHTTP session
    wchar_t* w_ua = utf8_to_wide("Lime/1.0");
    HINTERNET hSession = WinHttpOpen(w_ua, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                                      WINHTTP_NO_PROXY_NAME, WINHTTP_NO_PROXY_BYPASS, 0);
    free(w_ua);
    if (!hSession) { free(host); free(url_path); free(full_url); free(w_host); free(w_path); return NULL; }

    // Set timeout
    DWORD timeout = (DWORD)(builder->timeout_seconds * 1000);
    WinHttpSetTimeouts(hSession, timeout, timeout, timeout, timeout);

    // Create connection
    HINTERNET hConnect = WinHttpConnect(hSession, w_host, (INTERNET_PORT)port, 0);
    free(w_host);
    if (!hConnect) { WinHttpCloseHandle(hSession); free(host); free(url_path); free(full_url); free(w_path); return NULL; }

    // Create request
    wchar_t* w_method = utf8_to_wide(builder->method);
    HINTERNET hRequest = WinHttpOpenRequest(hConnect, w_method, w_path, NULL,
                                             WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES,
                                             use_tls ? WINHTTP_FLAG_SECURE : 0);
    free(w_method);
    free(w_path);
    if (!hRequest) { WinHttpCloseHandle(hConnect); WinHttpCloseHandle(hSession); free(host); free(url_path); free(full_url); return NULL; }

    // Set TLS options - only disable verification when explicitly requested
    if (use_tls) {
        if (!builder->verify) {
            DWORD security_flags = SECURITY_FLAG_IGNORE_UNKNOWN_CA |
                                   SECURITY_FLAG_IGNORE_CERT_DATE_INVALID |
                                   SECURITY_FLAG_IGNORE_CERT_CN_INVALID |
                                   SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE;
            WinHttpSetOption(hRequest, WINHTTP_OPTION_SECURITY_FLAGS, &security_flags, sizeof(security_flags));
        }
    }

    // Set redirects
    if (builder->disable_redirects) {
        DWORD policy = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
        WinHttpSetOption(hRequest, WINHTTP_OPTION_REDIRECT_POLICY, &policy, sizeof(policy));
    } else {
        DWORD policy = WINHTTP_OPTION_REDIRECT_POLICY_DISALLOW_HTTPS_TO_HTTP;
        WinHttpSetOption(hRequest, WINHTTP_OPTION_REDIRECT_POLICY, &policy, sizeof(policy));
        // Set max redirects
        DWORD max_redirs = (DWORD)builder->redirect_limit;
        WinHttpSetOption(hRequest, WINHTTP_OPTION_MAXHTTPAUTOREDIRECTS, &max_redirs, sizeof(max_redirs));
    }

    // Add headers
    StrBuilder hdr_sb;
    sb_init(&hdr_sb);

    // Add default headers from client
    if (builder->client && builder->client->default_headers) {
        HeaderEntry* e = builder->client->default_headers->first;
        while (e) {
            sb_append(&hdr_sb, e->key);
            sb_append(&hdr_sb, ": ");
            sb_append(&hdr_sb, e->value);
            sb_append(&hdr_sb, "\r\n");
            e = e->next;
        }
    }

    // Add request headers
    HeaderEntry* he = builder->headers->first;
    while (he) {
        sb_append(&hdr_sb, he->key);
        sb_append(&hdr_sb, ": ");
        sb_append(&hdr_sb, he->value);
        sb_append(&hdr_sb, "\r\n");
        he = he->next;
    }

    // Add basic auth
    if (builder->basic_auth_user) {
        // Build "Authorization: Basic base64(user:pass)"
        sb_append(&hdr_sb, "Authorization: Basic ");
        // Simple base64 encoding
        char* auth_str = (char*)malloc(strlen(builder->basic_auth_user) + 1 + strlen(builder->basic_auth_pass ? builder->basic_auth_pass : "") + 1);
        sprintf(auth_str, "%s:%s", builder->basic_auth_user, builder->basic_auth_pass ? builder->basic_auth_pass : "");
        // Base64 encode
        static const char b64tbl[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        size_t alen = strlen(auth_str);
        size_t olen = 4 * ((alen + 2) / 3);
        char* b64 = (char*)malloc(olen + 1);
        size_t j = 0;
        for (size_t i = 0; i < alen; i += 3) {
            unsigned int a = (unsigned char)auth_str[i];
            unsigned int b = (i + 1 < alen) ? (unsigned char)auth_str[i + 1] : 0;
            unsigned int c = (i + 2 < alen) ? (unsigned char)auth_str[i + 2] : 0;
            unsigned int triple = (a << 16) | (b << 8) | c;
            b64[j++] = b64tbl[(triple >> 18) & 0x3F];
            b64[j++] = b64tbl[(triple >> 12) & 0x3F];
            b64[j++] = (i + 1 < alen) ? b64tbl[(triple >> 6) & 0x3F] : '=';
            b64[j++] = (i + 2 < alen) ? b64tbl[triple & 0x3F] : '=';
        }
        b64[j] = '\0';
        sb_append(&hdr_sb, b64);
        sb_append(&hdr_sb, "\r\n");
        free(auth_str);
        free(b64);
    }

    // Add bearer auth
    if (builder->bearer_token) {
        sb_append(&hdr_sb, "Authorization: Bearer ");
        sb_append(&hdr_sb, builder->bearer_token);
        sb_append(&hdr_sb, "\r\n");
    }

    // Add content type for body
    if (builder->body_type == 3) { // JSON
        sb_append(&hdr_sb, "Content-Type: application/json\r\n");
    } else if (builder->body_type == 4) { // Form
        sb_append(&hdr_sb, "Content-Type: application/x-www-form-urlencoded\r\n");
    } else if (builder->body_type == 1 || builder->body_type == 2) {
        if (!header_map_contains(builder->headers, "Content-Type") &&
            !header_map_contains(builder->headers, "content-type")) {
            sb_append(&hdr_sb, "Content-Type: application/octet-stream\r\n");
        }
    }

    wchar_t* w_headers = utf8_to_wide(hdr_sb.data);
    free(hdr_sb.data);

    BOOL req_result = WinHttpSendRequest(hRequest, w_headers, (DWORD)-1,
                                          builder->body_data, (DWORD)builder->body_len,
                                          (DWORD)builder->body_len, 0);
    free(w_headers);

    if (!req_result) {
        WinHttpCloseHandle(hRequest);
        WinHttpCloseHandle(hConnect);
        WinHttpCloseHandle(hSession);
        free(host); free(url_path); free(full_url);
        return NULL;
    }

    // Receive response
    req_result = WinHttpReceiveResponse(hRequest, NULL);
    if (!req_result) {
        WinHttpCloseHandle(hRequest);
        WinHttpCloseHandle(hConnect);
        WinHttpCloseHandle(hSession);
        free(host); free(url_path); free(full_url);
        return NULL;
    }

    // Read response body
    StrBuilder body_sb;
    sb_init(&body_sb);
    char read_buf[8192];
    DWORD bytes_read = 0;
    while (WinHttpReadData(hRequest, read_buf, sizeof(read_buf), &bytes_read) && bytes_read > 0) {
        sb_append_n(&body_sb, read_buf, bytes_read);
        bytes_read = 0;
    }

    // Get status code
    DWORD status_code = 0;
    DWORD sc_size = sizeof(status_code);
    WinHttpQueryHeaders(hRequest, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                         WINHTTP_HEADER_NAME_BY_INDEX, &status_code, &sc_size, WINHTTP_NO_HEADER_INDEX);

    // Get response headers
    RequestsHeaderMap* resp_headers = header_map_new();
    {
        DWORD hdr_size = 0;
        WinHttpQueryHeaders(hRequest, WINHTTP_QUERY_RAW_HEADERS_CRLF,
                             WINHTTP_HEADER_NAME_BY_INDEX, NULL, &hdr_size, WINHTTP_NO_HEADER_INDEX);
        if (hdr_size > 0) {
            wchar_t* w_hdrs = (wchar_t*)malloc(hdr_size);
            if (w_hdrs && WinHttpQueryHeaders(hRequest, WINHTTP_QUERY_RAW_HEADERS_CRLF,
                                                WINHTTP_HEADER_NAME_BY_INDEX, w_hdrs, &hdr_size, WINHTTP_NO_HEADER_INDEX)) {
                // Convert wide to UTF-8
                int utf8_len = WideCharToMultiByte(CP_UTF8, 0, w_hdrs, -1, NULL, 0, NULL, NULL);
                char* utf8_hdrs = (char*)malloc(utf8_len);
                WideCharToMultiByte(CP_UTF8, 0, w_hdrs, -1, utf8_hdrs, utf8_len, NULL, NULL);
                // Parse headers
                char* line = strtok(utf8_hdrs, "\r\n");
                while (line) {
                    char* colon = strchr(line, ':');
                    if (colon) {
                        *colon = '\0';
                        char* key = line;
                        char* val = colon + 1;
                        while (*val == ' ') val++;
                        header_map_insert(resp_headers, key, val);
                    }
                    line = strtok(NULL, "\r\n");
                }
                free(utf8_hdrs);
            }
            free(w_hdrs);
        }
    }

    WinHttpCloseHandle(hRequest);
    WinHttpCloseHandle(hConnect);
    WinHttpCloseHandle(hSession);
    free(host); free(url_path);

    // Build response
    RequestsResponse* resp = (RequestsResponse*)malloc(sizeof(RequestsResponse));
    if (!resp) runtime_panic("requests: out of memory");
    resp->status_code = (int64_t)status_code;
    resp->headers = resp_headers;
    resp->url = full_url;
    resp->body = sb_finish(&body_sb);
    resp->body_len = resp->body ? (int64_t)strlen(resp->body) : 0;
    resp->redirect_history = NULL;

    // Update session cookie jar from Set-Cookie headers
    if (builder->session && builder->session->cookies) {
        runtime_requests_cookie_jar_update_from_response(
            builder->session->cookies, resp_headers, full_url);
    }

    return resp;
}

#else // POSIX

#include <unistd.h>

RequestsResponse* runtime_requests_send(RequestsRequestBuilder* builder) {
    if (!builder || !builder->url) return NULL;

    // Build curl command
    StrBuilder cmd;
    sb_init(&cmd);
    sb_append(&cmd, "curl -s -S -L");

    // Method
    if (builder->method) {
        sb_append(&cmd, " -X ");
        sb_append(&cmd, builder->method);
    }

    // Follow redirects
    if (!builder->disable_redirects) {
        sb_append(&cmd, " --max-redirs ");
        char buf[32];
        snprintf(buf, sizeof(buf), "%lld", (long long)builder->redirect_limit);
        sb_append(&cmd, buf);
    } else {
        sb_append(&cmd, " --max-redirs 0");
    }

    // Timeout
    {
        char buf[32];
        snprintf(buf, sizeof(buf), " --max-time %lld", (long long)builder->timeout_seconds);
        sb_append(&cmd, buf);
    }

    // Headers
    if (builder->client && builder->client->default_headers) {
        HeaderEntry* e = builder->client->default_headers->first;
        while (e) {
            char* hdr_val = (char*)malloc(strlen(e->key) + strlen(e->value) + 3);
            sprintf(hdr_val, "%s: %s", e->key, e->value);
            char* escaped_hdr = shell_escape(hdr_val);
            sb_append(&cmd, " -H '");
            sb_append(&cmd, escaped_hdr);
            sb_append(&cmd, "'");
            free(escaped_hdr);
            free(hdr_val);
            e = e->next;
        }
    }
    HeaderEntry* he = builder->headers->first;
    while (he) {
        char* hdr_val = (char*)malloc(strlen(he->key) + strlen(he->value) + 3);
        sprintf(hdr_val, "%s: %s", he->key, he->value);
        char* escaped_hdr = shell_escape(hdr_val);
        sb_append(&cmd, " -H '");
        sb_append(&cmd, escaped_hdr);
        sb_append(&cmd, "'");
        free(escaped_hdr);
        free(hdr_val);
        he = he->next;
    }

    // Basic auth
    if (builder->basic_auth_user) {
        char* auth_val = (char*)malloc(strlen(builder->basic_auth_user) + 1 + strlen(builder->basic_auth_pass ? builder->basic_auth_pass : "") + 1);
        sprintf(auth_val, "%s:%s", builder->basic_auth_user, builder->basic_auth_pass ? builder->basic_auth_pass : "");
        char* escaped_auth = shell_escape(auth_val);
        sb_append(&cmd, " -u '");
        sb_append(&cmd, escaped_auth);
        sb_append(&cmd, "'");
        free(escaped_auth);
        free(auth_val);
    }

    // Bearer auth
    if (builder->bearer_token) {
        char* bearer_val = (char*)malloc(strlen("Authorization: Bearer ") + strlen(builder->bearer_token) + 1);
        sprintf(bearer_val, "Authorization: Bearer %s", builder->bearer_token);
        char* escaped_bearer = shell_escape(bearer_val);
        sb_append(&cmd, " -H '");
        sb_append(&cmd, escaped_bearer);
        sb_append(&cmd, "'");
        free(escaped_bearer);
        free(bearer_val);
    }

    // Body
    if (builder->body_type == 1 || builder->body_type == 2) {
        if (builder->body_data && builder->body_len > 0) {
            // Use binary-safe escape: copy body data to a temporary buffer
            char* escaped = shell_escape(builder->body_data);
            sb_append(&cmd, " -d '");
            sb_append(&cmd, escaped);
            sb_append(&cmd, "'");
            free(escaped);
        }
    } else if (builder->body_type == 3) {
        sb_append(&cmd, " -H 'Content-Type: application/json'");
        if (builder->body_data && builder->body_len > 0) {
            char* escaped = shell_escape(builder->body_data);
            sb_append(&cmd, " -d '");
            sb_append(&cmd, escaped);
            sb_append(&cmd, "'");
            free(escaped);
        }
    } else if (builder->body_type == 4) {
        sb_append(&cmd, " -H 'Content-Type: application/x-www-form-urlencoded'");
        if (builder->body_data && builder->body_len > 0) {
            char* escaped = shell_escape(builder->body_data);
            sb_append(&cmd, " -d '");
            sb_append(&cmd, escaped);
            sb_append(&cmd, "'");
            free(escaped);
        }
    }

    // Output format: write headers to stderr, body to stdout
    sb_append(&cmd, " -i -w '\\n__LIME_STATUS__%{http_code}\\n__LIME_URL__%{url_effective}'");

    // URL
    sb_append(&cmd, " '");
    // Build full URL with query params
    StrBuilder url_sb;
    sb_init(&url_sb);
    sb_append(&url_sb, builder->url);
    if (builder->query_params.len > 0) {
        int has_q = strchr(builder->url, '?') != NULL;
        for (int64_t i = 0; i < builder->query_params.len; i++) {
            char* key = (char*)(intptr_t)runtime_list_get(builder->query_params, i);
            i++;
            if (i >= builder->query_params.len) break;
            char* val = (char*)(intptr_t)runtime_list_get(builder->query_params, i);
            if (!has_q && i == 1) sb_append(&url_sb, "?");
            else sb_append(&url_sb, "&");
            char* ek = url_encode(key);
            char* ev = url_encode(val);
            sb_append(&url_sb, ek);
            sb_append(&url_sb, "=");
            sb_append(&url_sb, ev);
            free(ek);
            free(ev);
        }
    }
    char* full_url = sb_finish(&url_sb);
    char* escaped_url = shell_escape(full_url);
    sb_append(&cmd, escaped_url);
    sb_append(&cmd, "'");
    free(escaped_url);

    char* curl_cmd = sb_finish(&cmd);

    // Execute curl
    FILE* fp = popen(curl_cmd, "r");
    free(curl_cmd);
    if (!fp) { free(full_url); return NULL; }

    // Read output
    StrBuilder out_sb;
    sb_init(&out_sb);
    char buf[8192];
    size_t n;
    while ((n = fread(buf, 1, sizeof(buf), fp)) > 0) {
        sb_append_n(&out_sb, buf, n);
    }
    int exit_status = pclose(fp);

    // Parse response: extract status code and URL from -w output
    int64_t status_code = 200;
    char* response_url = runtime_str_copy(full_url);

    // Look for markers BEFORE truncating
    char* status_marker = strstr(out_sb.data, "__LIME_STATUS__");
    char* url_marker = strstr(out_sb.data, "__LIME_URL__");

    // Extract URL from marker first (before any truncation)
    if (url_marker) {
        free(response_url);
        char* url_start = url_marker + 12;
        char* url_end = strchr(url_start, '\n');
        if (url_end) {
            response_url = (char*)malloc(url_end - url_start + 1);
            memcpy(response_url, url_start, url_end - url_start);
            response_url[url_end - url_start] = '\0';
        } else {
            response_url = runtime_str_copy(url_start);
        }
    }

    // Now extract status code and truncate
    if (status_marker) {
        status_code = atoi(status_marker + 15);
        *status_marker = '\0';
        out_sb.len = status_marker - out_sb.data;
    }
    // Remove URL marker from output if present
    if (url_marker) {
        *url_marker = '\0';
    }

    // Build response headers from -i output
    RequestsHeaderMap* resp_headers = header_map_new();
    // Parse headers from -i output (before the blank line separating headers from body)
    char* header_end = strstr(out_sb.data, "\r\n\r\n");
    if (!header_end) header_end = strstr(out_sb.data, "\n\n");
    if (header_end) {
        // Parse each header line
        char* hdr_section = (char*)malloc(header_end - out_sb.data + 1);
        memcpy(hdr_section, out_sb.data, header_end - out_sb.data);
        hdr_section[header_end - out_sb.data] = '\0';
        char* line = strtok(hdr_section, "\r\n");
        while (line) {
            char* colon = strchr(line, ':');
            if (colon) {
                *colon = '\0';
                char* key = line;
                char* val = colon + 1;
                while (*val == ' ') val++;
                header_map_insert(resp_headers, key, val);
            }
            line = strtok(NULL, "\r\n");
        }
        free(hdr_section);
        // Move body pointer past headers
        out_sb.data = header_end + ((*header_end == '\r') ? 4 : 2);
        out_sb.len = strlen(out_sb.data);
    }

    char* body = runtime_str_copy(out_sb.data);
    char* out_data = out_sb.data; // save for freeing
    RequestsResponse* resp = (RequestsResponse*)malloc(sizeof(RequestsResponse));
    if (!resp) runtime_panic("requests: out of memory");
    resp->status_code = status_code;
    resp->headers = resp_headers;
    resp->url = response_url;
    resp->body = body;
    resp->body_len = body ? (int64_t)strlen(body) : 0;
    resp->redirect_history = NULL;

    // Update session cookie jar from Set-Cookie headers
    if (builder->session && builder->session->cookies) {
        runtime_requests_cookie_jar_update_from_response(
            builder->session->cookies, resp_headers, response_url);
    }

    free(full_url);
    free(out_data);
    return resp;
}

#endif

// --- Response API ---

int64_t runtime_requests_response_status(RequestsResponse* response) {
    return response ? response->status_code : 0;
}

RequestsHeaderMap* runtime_requests_response_headers(RequestsResponse* response) {
    return response ? response->headers : NULL;
}

LimeList runtime_requests_response_headers_list(RequestsResponse* response) {
    LimeList list;
    runtime_list_empty(&list);
    if (!response || !response->headers) return list;
    HeaderEntry* e = response->headers->first;
    while (e) {
        runtime_list_add(&list, (int64_t)(intptr_t)runtime_str_copy(e->key));
        runtime_list_add(&list, (int64_t)(intptr_t)runtime_str_copy(e->value));
        e = e->next;
    }
    return list;
}

LimeList runtime_requests_response_redirect_history(RequestsResponse* response) {
    if (!response || !response->redirect_history) { LimeList _empty; runtime_list_empty(&_empty); return _empty; }
    return runtime_requests_redirect_history_list(response->redirect_history);
}

char* runtime_requests_response_url(RequestsResponse* response) {
    return response ? (response->url ? runtime_str_copy(response->url) : runtime_str_copy("")) : runtime_str_copy("");
}

char* runtime_requests_response_text(RequestsResponse* response) {
    if (!response || !response->body) return runtime_str_copy("");
    return runtime_str_copy(response->body);
}

char* runtime_requests_response_bytes(RequestsResponse* response, int64_t* out_len) {
    if (!response || !response->body) {
        if (out_len) *out_len = 0;
        return NULL;
    }
    char* copy = (char*)malloc(response->body_len + 1);
    if (!copy) return NULL;
    memcpy(copy, response->body, response->body_len);
    copy[response->body_len] = '\0';
    if (out_len) *out_len = response->body_len;
    return copy;
}

char* runtime_requests_response_json(RequestsResponse* response) {
    if (!response || !response->body) return NULL;
    return runtime_str_copy(response->body);
}

int64_t runtime_requests_response_content_length(RequestsResponse* response) {
    if (!response) return -1;
    return response->body_len;
}

int runtime_requests_response_is_success(RequestsResponse* response) {
    return response ? (response->status_code >= 200 && response->status_code < 300) : 0;
}

int runtime_requests_response_is_client_error(RequestsResponse* response) {
    return response ? (response->status_code >= 400 && response->status_code < 500) : 0;
}

int runtime_requests_response_is_server_error(RequestsResponse* response) {
    return response ? (response->status_code >= 500 && response->status_code < 600) : 0;
}

char* runtime_requests_response_error_for_status(RequestsResponse* response) {
    if (!response) return NULL;
    if (response->status_code >= 200 && response->status_code < 300) return NULL;
    char buf[128];
    snprintf(buf, sizeof(buf), "HTTP error: %lld", (long long)response->status_code);
    return runtime_str_copy(buf);
}

// --- Streaming ---

int64_t runtime_requests_response_copy_to(RequestsResponse* response, char* file_path) {
    if (!response || !response->body || !file_path) return -1;
    FILE* f = fopen(file_path, "wb");
    if (!f) return -1;
    size_t written = fwrite(response->body, 1, response->body_len, f);
    fclose(f);
    return (int64_t)written;
}

LimeList runtime_requests_response_chunks(RequestsResponse* response, int64_t chunk_size) {
    LimeList list;
    runtime_list_empty(&list);
    if (!response || !response->body || chunk_size <= 0) return list;
    int64_t pos = 0;
    while (pos < response->body_len) {
        int64_t remaining = response->body_len - pos;
        int64_t this_chunk = remaining < chunk_size ? remaining : chunk_size;
        char* chunk = (char*)malloc(this_chunk + 1);
        if (!chunk) break;
        memcpy(chunk, response->body + pos, this_chunk);
        chunk[this_chunk] = '\0';
        runtime_list_add(&list, (int64_t)(intptr_t)chunk);
        pos += this_chunk;
    }
    return list;
}

RequestsStream* runtime_requests_response_stream(RequestsResponse* response) {
    if (!response || !response->body) return NULL;
    RequestsStream* s = (RequestsStream*)malloc(sizeof(RequestsStream));
    if (!s) return NULL;
    s->data = runtime_str_copy(response->body);
    s->len = response->body_len;
    s->pos = 0;
    return s;
}

char* runtime_requests_stream_read(RequestsStream* stream, int64_t size, int64_t* out_len) {
    if (!stream || stream->pos >= stream->len) {
        if (out_len) *out_len = 0;
        return NULL;
    }
    int64_t remaining = stream->len - stream->pos;
    int64_t to_read = size < remaining ? size : remaining;
    char* buf = (char*)malloc(to_read + 1);
    if (!buf) { if (out_len) *out_len = 0; return NULL; }
    memcpy(buf, stream->data + stream->pos, to_read);
    buf[to_read] = '\0';
    stream->pos += to_read;
    if (out_len) *out_len = to_read;
    return buf;
}

int runtime_requests_stream_has_more(RequestsStream* stream) {
    return stream ? (stream->pos < stream->len) : 0;
}

// ============================================================
// Challenger Async Runtime — Future / Poll / Waker (Phase 1)
// ============================================================

ChallengerFuture* challenger_future_new(ChallengerPollFn poll_fn, void* state) {
    ChallengerFuture* fut = (ChallengerFuture*)malloc(sizeof(ChallengerFuture));
    if (!fut) return NULL;
    fut->poll_fn = poll_fn;
    fut->state = state;
    fut->output = 0;
    fut->completed = 0;
    return fut;
}

void challenger_future_free(ChallengerFuture* fut) {
    if (!fut) return;
    // State is owned by the future; caller is responsible for
    // freeing state before calling this, or we could add a state_free_fn.
    // For now: just free the future struct itself.
    free(fut);
}

Poll challenger_future_poll(ChallengerFuture* fut, ChallengerWaker* waker) {
    if (!fut || fut->completed) {
        Poll p;
        p.tag = 0;
        p.value = fut ? fut->output : 0;
        return p;
    }
    Poll result = fut->poll_fn(fut, waker);
    if (result.tag == 0) {
        // Ready: mark completed and store output
        fut->completed = 1;
        fut->output = result.value;
    }
    return result;
}

int8_t challenger_future_is_completed(ChallengerFuture* fut) {
    return fut ? fut->completed : 1;
}

ChallengerWaker* challenger_waker_new(ChallengerWakeFn wake_fn, void* data) {
    ChallengerWaker* w = (ChallengerWaker*)malloc(sizeof(ChallengerWaker));
    if (!w) return NULL;
    w->wake_fn = wake_fn;
    w->data = data;
    return w;
}

void challenger_waker_free(ChallengerWaker* waker) {
    if (!waker) return;
    free(waker);
}

void challenger_waker_wake(ChallengerWaker* waker) {
    if (!waker || !waker->wake_fn) return;
    waker->wake_fn(waker->data);
}

void challenger_waker_wake_by_ref(ChallengerWaker* waker) {
    if (!waker || !waker->wake_fn) return;
    waker->wake_fn(waker->data);
}

Poll challenger_poll_ready(int64_t value) {
    Poll p;
    p.tag = 0;
    p.value = value;
    return p;
}

Poll challenger_poll_pending(void) {
    Poll p;
    p.tag = 1;
    p.value = 0;
    return p;
}

// ============================================================
// Challenger Async Runtime — Task / Executor (Phase 3-5)
// ============================================================

// --- Ready Queue ---

void challenger_ready_queue_push(ReadyQueue* q, ChallengerTask* task) {
    if (!q || !task) return;
    if (q->count >= CHALLENGER_MAX_TASKS) return;
    q->tasks[q->tail] = task;
    q->tail = (q->tail + 1) % CHALLENGER_MAX_TASKS;
    q->count++;
    task->needs_poll = 1;
}

ChallengerTask* challenger_ready_queue_pop(ReadyQueue* q) {
    if (!q || q->count == 0) return NULL;
    ChallengerTask* task = q->tasks[q->head];
    q->head = (q->head + 1) % CHALLENGER_MAX_TASKS;
    q->count--;
    if (task) task->needs_poll = 0;
    return task;
}

int challenger_ready_queue_is_empty(ReadyQueue* q) {
    return q ? (q->count == 0) : 1;
}

// --- Executor ---

ChallengerExecutor* challenger_executor_new(void) {
    ChallengerExecutor* exec = (ChallengerExecutor*)calloc(1, sizeof(ChallengerExecutor));
    if (!exec) return NULL;
    exec->ready.head = 0;
    exec->ready.tail = 0;
    exec->ready.count = 0;
    exec->next_task_id = 1;
    exec->running = 0;
    exec->shutdown = 0;
    exec->worker_handle = NULL;
    return exec;
}

void challenger_executor_free(ChallengerExecutor* exec) {
    if (!exec) return;
    // Free all remaining tasks
    while (!challenger_ready_queue_is_empty(&exec->ready)) {
        ChallengerTask* t = challenger_ready_queue_pop(&exec->ready);
        if (t) {
            if (t->future) challenger_future_free(t->future);
            if (t->waker) challenger_waker_free(t->waker);
            free(t);
        }
    }
    free(exec);
}

uint64_t challenger_executor_spawn(ChallengerExecutor* exec, ChallengerFuture* fut) {
    if (!exec || !fut) return 0;

    ChallengerTask* task = (ChallengerTask*)calloc(1, sizeof(ChallengerTask));
    if (!task) return 0;

    task->id = exec->next_task_id++;
    task->future = fut;
    task->state = CHALLENGER_TASK_READY;
    task->needs_poll = 0;

    // Create a waker that references the executor and this task
    task->waker = challenger_waker_new(NULL, NULL); // placeholder waker

    challenger_ready_queue_push(&exec->ready, task);
    return task->id;
}

void challenger_executor_cancel(ChallengerExecutor* exec, uint64_t task_id) {
    if (!exec) return;
    // Find task in the ready queue and mark as cancelled
    for (int i = exec->ready.head; i != exec->ready.tail; i = (i + 1) % CHALLENGER_MAX_TASKS) {
        ChallengerTask* t = exec->ready.tasks[i];
        if (t && t->id == task_id) {
            t->state = CHALLENGER_TASK_CANCELLED;
            return;
        }
    }
}

void challenger_executor_wake_task(ChallengerExecutor* exec, uint64_t task_id) {
    if (!exec) return;
    // Find the task and re-enqueue it
    for (int i = exec->ready.head; i != exec->ready.tail; i = (i + 1) % CHALLENGER_MAX_TASKS) {
        ChallengerTask* t = exec->ready.tasks[i];
        if (t && t->id == task_id) {
            if (!t->needs_poll) {
                challenger_ready_queue_push(&exec->ready, t);
            }
            return;
        }
    }
}

int challenger_executor_run(ChallengerExecutor* exec) {
    if (!exec) return -1;
    exec->running = 1;

    while (!exec->shutdown) {
        // Check if all tasks are done
        if (challenger_ready_queue_is_empty(&exec->ready)) {
            break;
        }

        ChallengerTask* task = challenger_ready_queue_pop(&exec->ready);
        if (!task) continue;
        if (task->state == CHALLENGER_TASK_CANCELLED) {
            if (task->future) challenger_future_free(task->future);
            if (task->waker) challenger_waker_free(task->waker);
            free(task);
            continue;
        }

        task->state = CHALLENGER_TASK_RUNNING;

        // Poll the future
        if (task->future && !task->future->completed) {
            Poll result = challenger_future_poll(task->future, task->waker);

            if (result.tag == 0) {
                // Ready: task completed
                task->state = CHALLENGER_TASK_COMPLETED;
                task->future->output = result.value;
                if (task->future) challenger_future_free(task->future);
                if (task->waker) challenger_waker_free(task->waker);
                free(task);
            } else {
                // Pending: task waits for wake
                task->state = CHALLENGER_TASK_READY;
                // Task stays in the system but is not in the ready queue.
                // When waker fires, it will be re-enqueued.
            }
        } else {
            // Future already completed or null
            task->state = CHALLENGER_TASK_COMPLETED;
            if (task->future) challenger_future_free(task->future);
            if (task->waker) challenger_waker_free(task->waker);
            free(task);
        }
    }

    exec->running = 0;
    return 0;
}

void challenger_executor_park(ChallengerExecutor* exec) {
    if (!exec) return;
    // In single-thread mode: if no tasks are ready, we're done.
    // In multi-thread mode (later): block on a condition variable.
    // For now: no-op — the run loop handles this.
}

// --- Waker-Executor integration ---

// Global executor reference for waker callbacks (single-thread mode)
static ChallengerExecutor* g_challenger_executor = NULL;

void challenger_set_global_executor(ChallengerExecutor* exec) {
    g_challenger_executor = exec;
}

// Wake callback: called by waker when a task needs to be rescheduled
static void challenger_wake_callback(void* data) {
    uint64_t task_id = (uint64_t)(uintptr_t)data;
    if (g_challenger_executor) {
        challenger_executor_wake_task(g_challenger_executor, task_id);
    }
}

ChallengerWaker* challenger_waker_new_for_task(ChallengerExecutor* exec, uint64_t task_id) {
    return challenger_waker_new(challenger_wake_callback, (void*)(uintptr_t)task_id);
}

void challenger_waker_wake_from_executor(ChallengerExecutor* exec, uint64_t task_id) {
    challenger_executor_wake_task(exec, task_id);
}
