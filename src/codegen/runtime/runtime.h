// Lime Runtime Library - public declarations.
//
// These C types and functions back the LLVM runtime declarations the Lime
// codegen emits (see src/codegen/mod.rs). The ABI is fixed by the target
// object (test_print/runtime.obj) and docs/runtime.md:
//
//   LimeList  = { i8*, i64, i64 }  ->  { char* data; int64_t len; int64_t cap }
//   LimeOption= { i1, i8* }        ->  { int8_t has_value; void* value }
//   LimeIface = { i8*, i8* }       ->  { void* data; void* vtable }
//
// Functions taking/returning these structs by value follow the platform C ABI
// (on x86_64-windows-msvc the result is returned through a hidden pointer).

#ifndef LIME_RUNTIME_H
#define LIME_RUNTIME_H

#include <stdint.h>

typedef struct {
    char* data;   // pointer to element data (i64 elements)
    int64_t len;  // number of elements
    int64_t cap;  // capacity (number of elements)
} LimeList;

typedef struct {
    int8_t has_value; // 0 = None, 1 = Some
    void* value;      // pointer to value
} LimeOption;

typedef struct {
    void* data;   // pointer to struct data
    void* vtable; // interface vtable
} LimeIface;

// -- Allocation / control flow --
void* runtime_alloc(int64_t size, int64_t align);
void runtime_free(void* p);
void runtime_panic(char* msg);

// -- Print --
void runtime_print(char* s);

// -- String operations --
char* runtime_str_slice(char* s, int64_t start, int64_t end);
char* runtime_str_concat(char* a, char* b);
LimeList runtime_str_chars(char* s);
LimeList runtime_str_bytes(char* s);
int runtime_str_contains(char* s, char* sub);
int runtime_str_starts_with(char* s, char* prefix);
int runtime_str_ends_with(char* s, char* suffix);
char* runtime_str_trim(char* s);
char* runtime_str_replace(char* s, char* from, char* to);
LimeList runtime_str_split(char* s, char* sep);
char* runtime_str_to_upper(char* s);
char* runtime_str_to_lower(char* s);
char* runtime_str_repeat(char* s, int64_t times);

// -- Math --
double runtime_math_abs(double x);
double runtime_math_sqrt(double x);
double runtime_math_min(double a, double b);
double runtime_math_max(double a, double b);
double runtime_math_clamp(double x, double lo, double hi);
double runtime_math_pow(double a, double b);

// -- Time --
double runtime_time_now(void);
int runtime_time_sleep(double secs);

// -- stdio --
char* runtime_input(char* prompt);

// -- Filesystem --
char* runtime_read_file(char* path);
int runtime_write_file(char* path, char* content);
int runtime_append_file(char* path, char* content);
int runtime_file_exists(char* path);
int runtime_remove_file(char* path);
int runtime_fs_create_dir(char* path);
int64_t runtime_fs_size(char* path);
void runtime_fs_metadata(char* path, int64_t* size, int8_t* is_dir, int8_t* is_file);
LimeList runtime_fs_list_dir(char* path);

// -- List operations --
LimeList runtime_list_empty(void);
LimeList runtime_list_add(LimeList list, int64_t elem);
LimeList runtime_list_set(LimeList list, int64_t index, int64_t elem);

#endif // LIME_RUNTIME_H
