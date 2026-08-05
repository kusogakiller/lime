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
double runtime_math_floor(double x);
double runtime_math_ceil(double x);
double runtime_math_round(double x);
double runtime_math_trunc(double x);
double runtime_math_exp(double x);
double runtime_math_log(double x);
double runtime_math_log10(double x);
double runtime_math_sin(double x);
double runtime_math_cos(double x);
double runtime_math_tan(double x);
double runtime_math_asin(double x);
double runtime_math_acos(double x);
double runtime_math_atan(double x);
double runtime_math_pi(void);
double runtime_math_e(void);

// -- Time --
double runtime_time_now(void);
int runtime_time_sleep(double secs);

// -- stdio --
char* runtime_input(char* prompt);
void runtime_eprint(char* s);
void runtime_eprintln(char* s);
char* runtime_read_line(void);
char* runtime_read_all(void);
int runtime_write_stdout(char* s);
int runtime_write_stderr(char* s);

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

// -- JSON --
// Tagged union for JSON values.
// All values are passed as opaque i8* (LimeJson*) across the ABI.
typedef enum {
    JSON_NULL   = 0,
    JSON_BOOL   = 1,
    JSON_INT    = 2,
    JSON_FLOAT  = 3,
    JSON_STRING = 4,
    JSON_ARRAY  = 5,
    JSON_OBJECT = 6,
} LimeJsonTag;

typedef struct LimeJson {
    LimeJsonTag tag;
    union {
        int8_t   bool_val;
        int64_t  int_val;
        double   float_val;
        char*    string_val;
        struct { struct LimeJson** items; int64_t len; int64_t cap; } array_val;
        struct { char** keys; struct LimeJson** values; int64_t len; int64_t cap; } object_val;
    } data;
} LimeJson;

char*     runtime_json_stringify(LimeJson* j);
LimeJson* runtime_json_parse(char* s);
LimeJson* runtime_json_get(LimeJson* j, char* key);
int8_t    runtime_json_has(LimeJson* j, char* key);
int64_t   runtime_json_len(LimeJson* j);
LimeJson* runtime_json_at(LimeJson* j, int64_t index);
char*     runtime_json_as_string(LimeJson* j);
int64_t   runtime_json_as_int(LimeJson* j);
double    runtime_json_as_float(LimeJson* j);
int8_t    runtime_json_as_bool(LimeJson* j);
LimeJson* runtime_json_null(void);
LimeJson* runtime_json_object(void);
LimeJson* runtime_json_array(void);
int8_t    runtime_json_set(LimeJson* j, char* key, LimeJson* val);
int8_t    runtime_json_push(LimeJson* j, LimeJson* elem);

// -- Path operations (Phase C-1.8) --
char* runtime_path_join(char* a, char* b);
char* runtime_path_basename(char* path);
char* runtime_path_dirname(char* path);
char* runtime_path_filename(char* path);
char* runtime_path_extension(char* path);
int runtime_path_is_absolute(char* path);
char* runtime_path_normalize(char* path);
int runtime_path_equals(char* a, char* b);
char* runtime_path_parent(char* path);

// -- OS operations (Phase C-1.9) --
char* runtime_os_name(void);
char* runtime_os_arch(void);
char* runtime_os_platform(void);
char* runtime_os_hostname(void);
char* runtime_os_cwd(void);
int runtime_os_set_cwd(char* path);

// -- ENV operations (Phase C-1.9) --
char* runtime_env_get(char* key);
int runtime_env_has(char* key);
int runtime_env_set(char* key, char* value);
int runtime_env_remove(char* key);
LimeMap runtime_env_all(void);

// -- Regex operations (Phase C-1.10) --
char* runtime_regex_compile(char* pattern);
int runtime_regex_is_match(char* compiled, char* text);
char* runtime_regex_find(char* compiled, char* text);
LimeList runtime_regex_find_all(char* compiled, char* text);
char* runtime_regex_replace(char* compiled, char* text, char* replacement);
char* runtime_regex_replace_all(char* compiled, char* text, char* replacement);
LimeList runtime_regex_split(char* compiled, char* text);

// -- Process operations (Phase C-1.11) --
// Spawns a process without waiting. Returns PID or -1 on error.
int64_t runtime_process_spawn(char* command, LimeList args);
// Runs a process and waits for completion. Returns malloc'd stdout string.
char* runtime_process_run(char* command, LimeList args);
// Runs a process and returns stdout only. Returns malloc'd string.
char* runtime_process_output(char* command, LimeList args);
// Waits for a process to exit. Returns exit code or -1 on error.
int64_t runtime_process_wait(int64_t pid);
// Terminates a process. Returns 1 on success, 0 on failure.
int runtime_process_kill(int64_t pid);
// Returns process status as a malloc'd string: "running", "exited", or "failed".
char* runtime_process_status(int64_t pid);
// Returns the current program arguments as a LimeList of strings.
LimeList runtime_process_args(void);

// -- Requests operations (Phase C-1.12) --
// Opaque handle for HTTP client
typedef struct RequestsClient RequestsClient;
// Opaque handle for request builder
typedef struct RequestsRequestBuilder RequestsRequestBuilder;
// Opaque handle for response
typedef struct RequestsResponse RequestsResponse;
// Opaque handle for header map
typedef struct RequestsHeaderMap RequestsHeaderMap;
// Opaque handle for multipart form
typedef struct RequestsMultipart RequestsMultipart;
// Opaque handle for TLS config
typedef struct RequestsTlsConfig RequestsTlsConfig;
// Opaque handle for cookie jar
typedef struct RequestsCookieJar RequestsCookieJar;
// Opaque handle for stream
typedef struct RequestsStream RequestsStream;
// Opaque handle for HTTP session
typedef struct RequestsSession RequestsSession;
// Opaque handle for redirect history
typedef struct RequestsRedirectHistory RequestsRedirectHistory;

// Creates a new HTTP client with default configuration.
// Returns opaque pointer or NULL on error.
RequestsClient* runtime_requests_client_new(void);

// Creates a new HTTP client builder.
RequestsClient* runtime_requests_client_builder_new(void);

// Builds the client from the builder.
// Returns client pointer or NULL on error.
RequestsClient* runtime_requests_client_builder_build(RequestsClient* builder);

// Sets default headers on the client builder.
void runtime_requests_client_builder_default_headers(RequestsClient* builder, RequestsHeaderMap* headers);

// Sets default timeout on the client builder (in seconds).
void runtime_requests_client_builder_timeout(RequestsClient* builder, int64_t seconds);

// Sets redirect limit on the client builder.
void runtime_requests_client_builder_redirect_limit(RequestsClient* builder, int64_t limit);

// Disables redirects on the client builder.
void runtime_requests_client_builder_redirect_disabled(RequestsClient* builder);

// Sets proxy on the client builder.
void runtime_requests_client_builder_proxy(RequestsClient* builder, char* proxy_url);

// Sets TLS config on the client builder.
void runtime_requests_client_builder_tls_config(RequestsClient* builder, RequestsTlsConfig* tls_config);

// Creates a new request builder.
// Returns opaque pointer or NULL on error.
RequestsRequestBuilder* runtime_requests_request_builder_new(RequestsClient* client, char* method, char* url);

// Sets a header on the request builder.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_header(RequestsRequestBuilder* builder, char* key, char* value);

// Sets multiple headers on the request builder.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_headers(RequestsRequestBuilder* builder, RequestsHeaderMap* headers);

// Sets query parameters on the request builder.
// params is a LimeList of tuples (key, value).
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_query(RequestsRequestBuilder* builder, LimeList params);

// Sets the request body as bytes.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_body_bytes(RequestsRequestBuilder* builder, char* data, int64_t len);

// Sets the request body as a string.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_body_str(RequestsRequestBuilder* builder, char* body);

// Sets the request body as JSON.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_json(RequestsRequestBuilder* builder, void* json_value);

// Sets the request body as form data.
// data is a LimeList of tuples (key, value).
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_form(RequestsRequestBuilder* builder, LimeList data);

// Sets the request body as multipart.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_multipart(RequestsRequestBuilder* builder, RequestsMultipart* multipart);

// Sets the request timeout in seconds.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_timeout(RequestsRequestBuilder* builder, int64_t seconds);

// Sets the maximum number of redirects to follow.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_redirect_limit(RequestsRequestBuilder* builder, int64_t limit);

// Disables automatic redirect following.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_redirect_disabled(RequestsRequestBuilder* builder);

// Sets basic authentication.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_basic_auth(RequestsRequestBuilder* builder, char* user, char* password);

// Sets bearer token authentication.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_bearer_auth(RequestsRequestBuilder* builder, char* token);

// Sends the request and returns the response.
// Returns opaque pointer or NULL on error.
RequestsResponse* runtime_requests_send(RequestsRequestBuilder* builder);

// Returns the status code of the response.
int64_t runtime_requests_response_status(RequestsResponse* response);

// Returns the headers of the response.
RequestsHeaderMap* runtime_requests_response_headers(RequestsResponse* response);

// Returns the final URL after redirects.
// Returns malloc'd string.
char* runtime_requests_response_url(RequestsResponse* response);

// Returns the response body as a string.
// Returns malloc'd string or NULL on error.
char* runtime_requests_response_text(RequestsResponse* response);

// Returns the response body as bytes.
// Returns malloc'd buffer (caller must free) or NULL on error.
// Output length is stored in out_len.
char* runtime_requests_response_bytes(RequestsResponse* response, int64_t* out_len);

// Parses the response body as JSON.
// Returns malloc'd JSON string or NULL on error.
char* runtime_requests_response_json(RequestsResponse* response);

// Returns the content length of the response, or -1 if not available.
int64_t runtime_requests_response_content_length(RequestsResponse* response);

// Returns true if the status code is successful (200-299).
int runtime_requests_response_is_success(RequestsResponse* response);

// Returns true if the status code is a client error (400-499).
int runtime_requests_response_is_client_error(RequestsResponse* response);

// Returns true if the status code is a server error (500-599).
int runtime_requests_response_is_server_error(RequestsResponse* response);

// Returns an error message if the status code is not successful, or NULL if successful.
// Returns malloc'd string or NULL.
char* runtime_requests_response_error_for_status(RequestsResponse* response);

// Returns the numeric status code.
int64_t runtime_requests_status_code_code(int64_t code);

// Returns true if the status code is successful (200-299).
int runtime_requests_status_code_is_success(int64_t code);

// Returns true if the status code is a client error (400-499).
int runtime_requests_status_code_is_client_error(int64_t code);

// Returns true if the status code is a server error (500-599).
int runtime_requests_status_code_is_server_error(int64_t code);

// Returns true if the status code is a redirect (300-399).
int runtime_requests_status_code_is_redirect(int64_t code);

// Creates a new header map.
// Returns opaque pointer or NULL on error.
RequestsHeaderMap* runtime_requests_header_map_new(void);

// Inserts a header into the map.
// Returns 0 on success, -1 on error.
int runtime_requests_header_map_insert(RequestsHeaderMap* map, char* key, char* value);

// Appends a header value (allows multiple values for same key).
// Returns 0 on success, -1 on error.
int runtime_requests_header_map_append(RequestsHeaderMap* map, char* key, char* value);

// Removes a header from the map.
// Returns 0 on success, -1 on error.
int runtime_requests_header_map_remove(RequestsHeaderMap* map, char* key);

// Gets a header value by key.
// Returns malloc'd string or NULL if not found.
char* runtime_requests_header_map_get(RequestsHeaderMap* map, char* key);

// Checks if a header exists.
int runtime_requests_header_map_contains(RequestsHeaderMap* map, char* key);

// Creates a new multipart form.
// Returns opaque pointer or NULL on error.
RequestsMultipart* runtime_requests_multipart_new(void);

// Adds a text field to multipart form.
// Returns 0 on success, -1 on error.
int runtime_requests_multipart_text(RequestsMultipart* multipart, char* name, char* value);

// Adds a file to multipart form.
// Returns 0 on success, -1 on error.
int runtime_requests_multipart_file(RequestsMultipart* multipart, char* name, char* file_path);

// Adds a file with custom filename and content type to multipart form.
// Returns 0 on success, -1 on error.
int runtime_requests_multipart_file_with_metadata(RequestsMultipart* multipart, char* name, char* file_path, char* filename, char* content_type);

// Creates a new TLS configuration.
// Returns opaque pointer or NULL on error.
RequestsTlsConfig* runtime_requests_tls_config_new(void);

// Adds a custom CA certificate from a PEM file.
// Returns 0 on success, -1 on error.
int runtime_requests_tls_config_add_ca_cert(RequestsTlsConfig* config, char* pem_path);

// Adds a client certificate from PEM files.
// Returns 0 on success, -1 on error.
int runtime_requests_tls_config_add_client_cert(RequestsTlsConfig* config, char* cert_path, char* key_path);

// Disables certificate verification (dangerous, for testing only).
// Returns 0 on success, -1 on error.
int runtime_requests_tls_config_danger_accept_invalid_certs(RequestsTlsConfig* config);

// Disables hostname verification (dangerous, for testing only).
// Returns 0 on success, -1 on error.
int runtime_requests_tls_config_danger_accept_invalid_hostnames(RequestsTlsConfig* config);

// Creates a new cookie jar.
// Returns opaque pointer or NULL on error.
RequestsCookieJar* runtime_requests_cookie_jar_new(void);

// Adds a cookie to the jar.
// Returns 0 on success, -1 on error.
int runtime_requests_cookie_jar_add(RequestsCookieJar* jar, char* cookie_str);

// Adds a parsed cookie to the jar.
// Returns 0 on success, -1 on error.
int runtime_requests_cookie_jar_add_parsed(RequestsCookieJar* jar, void* cookie);

// Update cookie jar from response Set-Cookie headers.
void runtime_requests_cookie_jar_update_from_response(RequestsCookieJar* jar, RequestsHeaderMap* resp_headers, char* request_url);

// Build Cookie header string from matching cookies. Returns malloc'd string or NULL.
char* runtime_requests_cookie_jar_get_cookie_header(RequestsCookieJar* jar, char* url);

// Get all cookies as list of alternating name, value strings.
LimeList runtime_requests_cookie_jar_get_all(RequestsCookieJar* jar);

// Get a specific cookie value by name. Returns malloc'd string or NULL.
char* runtime_requests_cookie_jar_get(RequestsCookieJar* jar, char* name);

// Parses a cookie from string.
// Returns malloc'd cookie string or NULL on error.
char* runtime_requests_cookie_parse(char* cookie_str);

// Streams the response body to a file.
// Returns bytes written or -1 on error.
int64_t runtime_requests_response_copy_to(RequestsResponse* response, char* file_path);

// Returns redirect history as LimeList of alternating url, status_code.
LimeList runtime_requests_response_redirect_history(RequestsResponse* response);

// Streams the response body in chunks.
// Returns LimeList of byte buffers or NULL on error.
LimeList runtime_requests_response_chunks(RequestsResponse* response, int64_t chunk_size);

// Reads the response body as a stream.
// Returns opaque stream pointer or NULL on error.
RequestsStream* runtime_requests_response_stream(RequestsResponse* response);

// Reads a chunk from the stream.
// Returns malloc'd buffer or NULL on error/EOF.
// Output length is stored in out_len.
char* runtime_requests_stream_read(RequestsStream* stream, int64_t size, int64_t* out_len);

// Checks if stream has more data.
int runtime_requests_stream_has_more(RequestsStream* stream);

// Creates a new HTTP session with persistent state.
// Returns opaque pointer or NULL on error.
RequestsSession* runtime_requests_session_new(void);

// Creates a request builder from a session.
// Returns opaque pointer or NULL on error.
RequestsRequestBuilder* runtime_requests_session_request(RequestsSession* session, char* method, char* url);

// Session setters for Python requests compatibility
int runtime_requests_session_set_default_headers(RequestsSession* session, LimeList headers);
int runtime_requests_session_set_default_params(RequestsSession* session, LimeList params);
int runtime_requests_session_set_timeout(RequestsSession* session, int64_t seconds);
int runtime_requests_session_set_verify(RequestsSession* session, int verify);
int runtime_requests_session_set_redirect_limit(RequestsSession* session, int64_t limit);
int runtime_requests_session_set_disable_redirects(RequestsSession* session, int disable);

// Get session cookies as list of alternating name, value strings.
LimeList runtime_requests_session_cookies(RequestsSession* session);

// Redirect history
RequestsRedirectHistory* runtime_requests_redirect_history_new(void);
void runtime_requests_redirect_history_add(RequestsRedirectHistory* history, int64_t status_code, char* url, char* method);
LimeList runtime_requests_redirect_history_list(RequestsRedirectHistory* history);
void runtime_requests_redirect_history_free(RequestsRedirectHistory* history);

// Sets headers from a flat list of key/value string pairs.
// list is a LimeList of char* strings in alternating key, value order.
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_set_headers(RequestsRequestBuilder* builder, LimeList headers);

// Sets whether to verify TLS certificates (1=verify, 0=don't verify).
// Returns 0 on success, -1 on error.
int runtime_requests_request_builder_verify(RequestsRequestBuilder* builder, int verify);

// Returns response headers as a LimeList of alternating key, value strings.
// Caller must free the returned list contents.
LimeList runtime_requests_response_headers_list(RequestsResponse* response);

// Frees a session.
void runtime_requests_session_free(RequestsSession* session);

// Frees a client.
void runtime_requests_client_free(RequestsClient* client);

// Frees a request builder.
void runtime_requests_request_builder_free(RequestsRequestBuilder* builder);

// Frees a response.
void runtime_requests_response_free(RequestsResponse* response);

// Frees a header map.
void runtime_requests_header_map_free(RequestsHeaderMap* map);

// Frees a multipart form.
void runtime_requests_multipart_free(RequestsMultipart* multipart);

// Frees a TLS config.
void runtime_requests_tls_config_free(RequestsTlsConfig* config);

// Frees a cookie jar.
void runtime_requests_cookie_jar_free(RequestsCookieJar* jar);

// Frees a stream.
void runtime_requests_stream_free(RequestsStream* stream);

#endif // LIME_RUNTIME_H
