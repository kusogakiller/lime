// Lime Runtime Library
// Link with compiled Lime programs for object/executable output.

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>

// LimeList structure matches the LLVM IR type %LimeList = type { i8*, i64, i64 }
typedef struct {
    char* data;       // pointer to element data
    int64_t len;      // number of elements
    int64_t cap;      // capacity
} LimeList;

// LimeOption structure matches %LimeOption = type { i1, i8* }
typedef struct {
    int8_t has_value; // 0 = None, 1 = Some
    void* value;      // pointer to value
} LimeOption;

// LimeIface structure matches %LimeIface = type { i8*, i8* }
typedef struct {
    void* data;       // pointer to struct data
    void* vtable;     // reserved for future dynamic dispatch
} LimeIface;

// -- Runtime allocation --
void* runtime_alloc(int64_t size, int64_t align) {
    (void)align;
    return malloc((size_t)size);
}

// -- Print --
void runtime_print(char* s) {
    puts(s);
}

// -- String operations --
// strlen is already a built-in in C's <string.h>
// We provide runtime_strlen as alias
int64_t runtime_strlen(char* s) {
    return (int64_t)strlen(s);
}

char* runtime_str_slice(char* s, int64_t start, int64_t end) {
    int64_t len = end - start;
    char* result = (char*)malloc((size_t)(len + 1));
    memcpy(result, s + start, (size_t)len);
    result[len] = '\0';
    return result;
}

char* runtime_str_concat(char* a, char* b) {
    size_t la = strlen(a);
    size_t lb = strlen(b);
    char* result = (char*)malloc(la + lb + 1);
    memcpy(result, a, la);
    memcpy(result + la, b, lb);
    result[la + lb] = '\0';
    return result;
}

LimeList runtime_str_chars(char* s) {
    int64_t len = (int64_t)strlen(s);
    LimeList list;
    list.data = (char*)malloc((size_t)(len * 8));
    list.len = len;
    list.cap = len;
    // Copy each byte as an element
    for (int64_t i = 0; i < len; i++) {
        // We need to store each char as a pointer to a 2-byte string
        // For simplicity, store the integer value in the element array
        ((int64_t*)list.data)[i] = (int64_t)(unsigned char)s[i];
    }
    return list;
}

LimeList runtime_str_bytes(char* s) {
    return runtime_str_chars(s);
}

// -- List operations --
LimeList runtime_list_add(LimeList list, int64_t elem) {
    if (list.len >= list.cap) {
        int64_t new_cap = list.cap == 0 ? 4 : list.cap * 2;
        list.data = (char*)realloc(list.data, (size_t)(new_cap * 8));
        list.cap = new_cap;
    }
    ((int64_t*)list.data)[list.len] = elem;
    list.len++;
    return list;
}

LimeList runtime_list_set(LimeList list, int64_t index, int64_t elem) {
    if (index >= 0 && index < list.len) {
        ((int64_t*)list.data)[index] = elem;
    }
    return list;
}
