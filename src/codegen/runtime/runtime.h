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

// Phase B-2.2: Closure / function values
// { fn_ptr, env_ptr }  — matches LLVM %LimeClosure = type { i8*, i8* }
typedef struct {
    void* fn_ptr;   // pointer to the function
    void* env_ptr;  // captured environment (NULL for plain fn refs)
} LimeClosure;

// -- Allocation / control flow --
void* runtime_alloc(int64_t size, int64_t align);
void runtime_free(void* p);
void runtime_panic(char* msg);

// -- Print --
void runtime_print(char* s);

// -- str() conversion helpers (Phase B-1) --
char* runtime_str_from_i64(int64_t v);
char* runtime_str_from_f64(double v);
char* runtime_str_from_bool(int8_t v);

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
int runtime_fs_copy(char* src, char* dst);
int runtime_fs_rename(char* src, char* dst);
int runtime_fs_is_file(char* path);
int runtime_fs_is_dir(char* path);
int runtime_fs_remove_dir(char* path);
LimeList runtime_fs_read_lines(char* path);
int runtime_fs_write_lines(char* path, LimeList lines);

// -- List operations --
LimeList runtime_list_empty(void);
LimeList runtime_list_add(LimeList list, int64_t elem);
LimeList runtime_list_set(LimeList list, int64_t index, int64_t elem);
int64_t runtime_list_len(LimeList list);
int64_t runtime_list_get(LimeList list, int64_t index);

// -- List mutation / inspection (Phase C-1.2) --
LimeList runtime_list_insert(LimeList list, int64_t index, int64_t elem);
LimeList runtime_list_clear(LimeList list);
LimeList runtime_list_sort(LimeList list);
LimeList runtime_list_clone(LimeList list);

// -- Map operations --
typedef struct {
    void* data;   // pointer to key-value pairs (i64 pairs: key, value interleaved)
    int64_t len;  // number of entries
    int64_t cap;  // capacity (number of entries)
} LimeMap;

int64_t runtime_map_len(LimeMap map);
int runtime_map_is_empty(LimeMap map);
LimeMap runtime_map_insert(LimeMap map, int64_t key, int64_t val);
int64_t runtime_map_get(LimeMap map, int64_t key);
LimeMap runtime_map_remove(LimeMap map, int64_t key);
int runtime_map_contains_key(LimeMap map, int64_t key);
LimeMap runtime_map_clear(LimeMap map);
LimeMap runtime_map_clone(LimeMap map);

// -- Set operations --
typedef struct {
    void* data;   // pointer to elements (i64 values)
    int64_t len;  // number of elements
    int64_t cap;  // capacity (number of elements)
} LimeSet;

int64_t runtime_set_len(LimeSet set);
int runtime_set_is_empty(LimeSet set);
LimeSet runtime_set_add(LimeSet set, int64_t elem);
LimeSet runtime_set_remove(LimeSet set, int64_t elem);
int runtime_set_contains(LimeSet set, int64_t elem);
LimeSet runtime_set_clear(LimeSet set);
LimeSet runtime_set_clone(LimeSet set);

// -- Queue operations (implemented on top of LimeList, FIFO) --
LimeList runtime_queue_push(LimeList queue, int64_t elem);
int64_t runtime_queue_pop(LimeList queue);
int64_t runtime_queue_front(LimeList queue);
int64_t runtime_queue_back(LimeList queue);
int64_t runtime_queue_len(LimeList queue);
int runtime_queue_is_empty(LimeList queue);
LimeList runtime_queue_clear(LimeList queue);

// -- Stack operations (implemented on top of LimeList, LIFO) --
LimeList runtime_stack_push(LimeList stack, int64_t elem);
int64_t runtime_stack_pop(LimeList stack);
int64_t runtime_stack_peek(LimeList stack);
int64_t runtime_stack_len(LimeList stack);
int runtime_stack_is_empty(LimeList stack);
LimeList runtime_stack_clear(LimeList stack);

// -- Closure / function values (Phase B-2.2 / B-2.3) --
//
// LimeClosure ABI:
//   %LimeClosure = type { i8* %fn_ptr, i8* %env_ptr }
//
//   fn_ptr: pointer to a function with signature i64(i8* %env, i8* %packed_args)
//           or i8*(i8* %env, i8* %packed_args) depending on return type.
//   env_ptr: pointer to captured environment (NULL for plain function references).
//            Always NULL in current implementation (no native capture yet).
//
//   Packed args: heap-allocated i64 array, one slot per argument.
//                The callee unpacks via GEP + bitcast + load.
//
//   Ownership: closure struct is heap-allocated; caller owns packed_args array.
//
LimeClosure* runtime_make_closure(void* fn_ptr, void* env_ptr);
int64_t runtime_call_closure_i64(LimeClosure* closure, void* packed_args);
void* runtime_call_closure_ptr(LimeClosure* closure, void* packed_args);
LimeClosure* runtime_make_fn_ref(void* fn_ptr);

#endif // LIME_RUNTIME_H
