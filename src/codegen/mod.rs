// Phase 0 (Step 10): LLVM backend foundation (textual IR emitter).
//
// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・LLVM IR (text) 縺ｦ榆帥蜴ｻ縺ｦ縺ｪ縺ｿ縺ｪ縺・
// Inkwell / llvm-sys 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・蟋ｩ蜉ｨ蝗ｲ繧縺ｮ縺ｿ縺ｪ縺・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
// (System LLVM 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・)
// 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｿ縺ｪ縺・(aggregates) 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
// 壓ｵ蜈ｰ縺ｯ縺ｪ縺ｿ縺ｪ縺・ Phase 1+ 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
//
// Design (docs/llvm_backend.md) 縺ｮ codegen/ 縺ｯ繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・:
//   mod.rs (Context/Module/Builder-like) / types.rs (型マッピング) /
//   fn_builder.rs / structs.rs / calls.rs / generic.rs / interface.rs /
//   async_rt.rs / runtime/ ... (Phase 1+ 縺ｯ頒ｦ)

pub mod fn_builder;
pub mod types;

use crate::Defs;
use crate::Expr;
use crate::FunctionDef;
use crate::MemoryPlace;
use crate::Stmt;
use crate::Type;
use crate::codegen::types::llvm_type_name;
use crate::type_from_str;
use std::collections::HashMap;

const DEFAULT_TARGET_TRIPLE: &str = "x86_64-pc-windows-msvc";
const LIME_MAIN: &str = "main_lime";

pub fn emit_llvm(
    stmts: &[Stmt],
    defs: &Defs,
    memory: &HashMap<String, MemoryPlace>,
) -> (String, Vec<String>) {
    let _ = stmts;
    let mut out = String::new();
    let mut warnings: Vec<String> = Vec::new();

    out.push_str("; ModuleID = 'lime'\n");
    out.push_str("source_filename = \"lime\"\n");
    out.push_str(&format!(
        "target triple = \"{}\"\n\n",
        DEFAULT_TARGET_TRIPLE
    ));

    // Runtime aggregate types must precede any declaration that uses them
    // (LLVM rejects forward-referenced struct types in function signatures).
    out.push_str("%LimeList = type { i8*, i64, i64 }\n");
    out.push_str("%LimeOption = type { i1, i8* }\n");
    out.push_str("declare void @llvm.memcpy.p0.p0.i64(i8*, i8*, i64, i1)\n");
    out.push_str("%LimeIface = type { i8*, i8* }\n");
    out.push_str("%LimeClosure = type { i8*, i8* }\n");
    out.push_str("%LimeMap = type { i8*, i64, i64 }\n");
    out.push_str("%LimeSet = type { i8*, i64, i64 }\n\n");

    // Runtime declarations
    out.push_str("declare i8* @runtime_alloc(i64, i64)\n");
    out.push_str("declare void @runtime_print(i8*)\n");
    // Printf for print/println builtin lowering (Phase 2)
    out.push_str("declare i32 @printf(i8*, ...)\n");
    out.push_str("declare void @runtime_panic(i8*)\n\n");

    // Phase 5: String/List runtime function declarations
    out.push_str("declare i64 @strlen(i8*)\n");
    out.push_str("declare i8* @runtime_str_slice(i8*, i64, i64)\n");
    out.push_str("declare i8* @runtime_str_concat(i8*, i8*)\n");
    out.push_str("declare void @runtime_str_chars(ptr sret(%LimeList), ptr)\n");
    out.push_str("declare void @runtime_str_bytes(ptr sret(%LimeList), ptr)\n");
    out.push_str("declare void @runtime_list_add(ptr, i64) alwaysinline\n");
    out.push_str("declare void @runtime_list_set(ptr, i64, i64)\n\n");
    out.push_str("declare void @runtime_list_empty(ptr)\n");
    out.push_str("declare i64 @runtime_list_len(%LimeList)\n");
    out.push_str("declare i64 @runtime_list_get(%LimeList, i64)\n\n");

    // Phase 12 Step 1: stdlib runtime builtins (string/math/time/fs/io)
    out.push_str("declare i32 @runtime_str_contains(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_str_starts_with(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_str_ends_with(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_str_trim(i8*)\n");
    out.push_str("declare i8* @runtime_str_replace(i8*, i8*, i8*)\n");
    out.push_str("declare void @runtime_str_split(ptr sret(%LimeList), ptr, ptr)\n");
    out.push_str("declare i8* @runtime_str_to_upper(i8*)\n");
    out.push_str("declare i8* @runtime_str_to_lower(i8*)\n");
    out.push_str("declare i8* @runtime_str_repeat(i8*, i64)\n");
    out.push_str("declare i8* @runtime_str_from_i64(i64)\n");
    out.push_str("declare i8* @runtime_str_from_f64(double)\n");
    out.push_str("declare i8* @runtime_str_from_bool(i1)\n");
    out.push_str(
        "declare i64 @runtime_str_byte(i8* nocapture readonly, i64) nounwind willreturn\n",
    );
    out.push_str("declare i8* @runtime_str_new(i64)\n");
    out.push_str("declare i8* @runtime_str_from_byte(i64)\n");
    out.push_str("declare i8* @runtime_str_push_byte(i8*, i64)\n");
    out.push_str("declare i8* @runtime_str_push_byte_len(i8*, i64, i64)\n");
    // Phase B-3: extended string builtins
    out.push_str("declare i32 @runtime_str_is_empty(i8*)\n");
    out.push_str("declare i64 @runtime_str_find(i8*, i8*)\n");
    out.push_str("declare i64 @runtime_str_count(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_str_trim_start(i8*)\n");
    out.push_str("declare i8* @runtime_str_trim_end(i8*)\n");
    out.push_str("declare i8* @runtime_str_join(ptr, i8*)\n");
    out.push_str("declare i64 @runtime_str_to_int(i8*)\n");
    out.push_str("declare double @runtime_str_to_float(i8*)\n");
    out.push_str("declare i32 @runtime_str_equals(i8* nocapture readonly, i8* nocapture readonly) nounwind willreturn\n");
    out.push_str("declare i32 @runtime_str_compare(i8*, i8*)\n");
    out.push_str("declare double @runtime_math_abs(double)\n");
    out.push_str("declare double @runtime_math_sqrt(double)\n");
    out.push_str("declare double @runtime_math_min(double, double)\n");
    out.push_str("declare double @runtime_math_max(double, double)\n");
    out.push_str("declare double @runtime_math_clamp(double, double, double)\n");
    out.push_str("declare double @runtime_math_pow(double, double)\n");
    out.push_str("declare double @runtime_math_floor(double)\n");
    out.push_str("declare double @runtime_math_ceil(double)\n");
    out.push_str("declare double @runtime_math_round(double)\n");
    out.push_str("declare double @runtime_math_trunc(double)\n");
    out.push_str("declare double @runtime_math_exp(double)\n");
    out.push_str("declare double @runtime_math_log(double)\n");
    out.push_str("declare double @runtime_math_log10(double)\n");
    out.push_str("declare double @runtime_math_sin(double)\n");
    out.push_str("declare double @runtime_math_cos(double)\n");
    out.push_str("declare double @runtime_math_tan(double)\n");
    out.push_str("declare double @runtime_math_asin(double)\n");
    out.push_str("declare double @runtime_math_acos(double)\n");
    out.push_str("declare double @runtime_math_atan(double)\n");
    out.push_str("declare double @runtime_math_pi()\n");
    out.push_str("declare double @runtime_math_e()\n");
    out.push_str("declare i8* @runtime_str_from_option(i64, i32)\n");
    out.push_str("declare i8* @runtime_str_from_result(i64, i32)\n");
    out.push_str("declare double @runtime_time_now()\n");
    out.push_str("declare i32 @runtime_time_sleep(double)\n");
    out.push_str("declare i8* @runtime_input(i8*)\n");
    out.push_str("declare void @runtime_eprint(i8*)\n");
    out.push_str("declare void @runtime_eprintln(i8*)\n");
    out.push_str("declare i8* @runtime_read_line()\n");
    out.push_str("declare i8* @runtime_read_all()\n");
    out.push_str("declare i32 @runtime_write_stdout(i8*)\n");
    out.push_str("declare i32 @runtime_write_stderr(i8*)\n");
    out.push_str("declare i8* @runtime_read_file(i8*)\n");
    out.push_str("declare i32 @runtime_write_file(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_append_file(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_file_exists(i8*)\n");
    out.push_str("declare i32 @runtime_remove_file(i8*)\n");
    out.push_str("declare i32 @runtime_fs_create_dir(i8*)\n");
    out.push_str("declare i64 @runtime_fs_size(i8*)\n");
    out.push_str("declare void @runtime_fs_metadata(i8*, ptr, ptr, ptr)\n");
    out.push_str("declare void @runtime_fs_list_dir(ptr sret(%LimeList), ptr)\n");
    out.push_str("declare i32 @runtime_fs_copy(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_fs_rename(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_fs_is_file(i8*)\n");
    out.push_str("declare i32 @runtime_fs_is_dir(i8*)\n");
    out.push_str("declare i32 @runtime_fs_remove_dir(i8*)\n");
    out.push_str("declare void @runtime_fs_read_lines(ptr sret(%LimeList), ptr)\n");
    out.push_str("declare i32 @runtime_fs_write_lines(i8*, ptr)\n");

    // Phase C-1.2: list mutation / inspection builtins
    out.push_str("declare void @runtime_list_insert(ptr, i64, i64)\n");
    out.push_str("declare void @runtime_list_clear(ptr)\n");
    out.push_str("declare void @runtime_list_sort(ptr)\n");
    out.push_str("declare void @runtime_list_clone(ptr, ptr)\n");

    // Phase C-1.2: map builtins
    out.push_str("declare i64 @runtime_map_len(ptr)\n");
    out.push_str("declare i32 @runtime_map_is_empty(ptr)\n");
    out.push_str("declare void @runtime_map_insert(ptr sret(%LimeMap), ptr, i64, i64)\n");
    out.push_str("declare i64 @runtime_map_get(ptr, i64)\n");
    out.push_str("declare void @runtime_map_remove(ptr sret(%LimeMap), ptr, i64)\n");
    out.push_str("declare i32 @runtime_map_contains_key(ptr, i64)\n");
    out.push_str("declare void @runtime_map_clear(ptr sret(%LimeMap), ptr)\n");
    out.push_str("declare void @runtime_map_clone(ptr sret(%LimeMap), ptr)\n");

    // Phase C-1.2: set builtins
    out.push_str("declare i64 @runtime_set_len(ptr)\n");
    out.push_str("declare i32 @runtime_set_is_empty(ptr)\n");
    out.push_str("declare void @runtime_set_add(ptr sret(%LimeSet), ptr, i64)\n");
    out.push_str("declare void @runtime_set_remove(ptr sret(%LimeSet), ptr, i64)\n");
    out.push_str("declare i32 @runtime_set_contains(ptr, i64)\n");
    out.push_str("declare void @runtime_set_clear(ptr sret(%LimeSet), ptr)\n");
    out.push_str("declare void @runtime_set_clone(ptr sret(%LimeSet), ptr)\n");

    // Phase C-1.2: queue builtins (FIFO, backed by LimeList)
    out.push_str("declare void @runtime_queue_push(ptr sret(%LimeList), ptr, i64)\n");
    out.push_str("declare i64 @runtime_queue_pop(ptr)\n");
    out.push_str("declare i64 @runtime_queue_front(ptr)\n");
    out.push_str("declare i64 @runtime_queue_back(ptr)\n");
    out.push_str("declare i64 @runtime_queue_len(ptr)\n");
    out.push_str("declare i32 @runtime_queue_is_empty(ptr)\n");
    out.push_str("declare void @runtime_queue_clear(ptr sret(%LimeList), ptr)\n");

    // Phase C-1.2: stack builtins (LIFO, backed by LimeList)
    out.push_str("declare void @runtime_stack_push(ptr sret(%LimeList), ptr, i64)\n");
    out.push_str("declare i64 @runtime_stack_pop(ptr)\n");
    out.push_str("declare i64 @runtime_stack_peek(ptr)\n");
    out.push_str("declare i64 @runtime_stack_len(ptr)\n");
    out.push_str("declare i32 @runtime_stack_is_empty(ptr)\n");
    out.push_str("declare void @runtime_stack_clear(ptr sret(%LimeList), ptr)\n");

    // Phase B-2.2: closure / function-value runtime helpers
    out.push_str("declare %LimeClosure* @runtime_make_closure(i8*, i8*)\n");
    out.push_str("declare i64 @runtime_call_closure_i64(%LimeClosure*, i8*)\n");
    out.push_str("declare i8* @runtime_call_closure_ptr(%LimeClosure*, i8*)\n");
    out.push_str("declare %LimeClosure* @runtime_make_fn_ref(i8*)\n\n");

    // JSON runtime declarations
    out.push_str("declare i8* @runtime_json_parse(i8*)\n");
    out.push_str("declare i8* @runtime_json_stringify(i8*)\n");
    out.push_str("declare i8* @runtime_json_get(i8*, i8*)\n");
    out.push_str("declare i8 @runtime_json_has(i8*, i8*)\n");
    out.push_str("declare i64 @runtime_json_len(i8*)\n");
    out.push_str("declare i8* @runtime_json_at(i8*, i64)\n");
    out.push_str("declare i8* @runtime_json_as_string(i8*)\n");
    out.push_str("declare i64 @runtime_json_as_int(i8*)\n");
    out.push_str("declare double @runtime_json_as_float(i8*)\n");
    out.push_str("declare i8 @runtime_json_as_bool(i8*)\n");
    out.push_str("declare i8* @runtime_json_null()\n");
    out.push_str("declare i8* @runtime_json_object()\n");
    out.push_str("declare i8* @runtime_json_array()\n");
    out.push_str("declare i8 @runtime_json_set(i8*, i8*, i8*)\n");
    out.push_str("declare i8 @runtime_json_push(i8*, i8*)\n\n");

    // Path runtime declarations (Phase C-1.8)
    out.push_str("declare i8* @runtime_path_join(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_path_basename(i8*)\n");
    out.push_str("declare i8* @runtime_path_dirname(i8*)\n");
    out.push_str("declare i8* @runtime_path_filename(i8*)\n");
    out.push_str("declare i8* @runtime_path_extension(i8*)\n");
    out.push_str("declare i32 @runtime_path_is_absolute(i8*)\n");
    out.push_str("declare i8* @runtime_path_normalize(i8*)\n");
    out.push_str("declare i32 @runtime_path_equals(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_path_parent(i8*)\n\n");

    // OS runtime declarations (Phase C-1.9)
    out.push_str("declare i8* @runtime_os_name()\n");
    out.push_str("declare i8* @runtime_os_arch()\n");
    out.push_str("declare i8* @runtime_os_platform()\n");
    out.push_str("declare i8* @runtime_os_hostname()\n");
    out.push_str("declare i8* @runtime_os_cwd()\n");
    out.push_str("declare i32 @runtime_os_set_cwd(i8*)\n\n");

    // ENV runtime declarations (Phase C-1.9)
    out.push_str("declare i8* @runtime_env_get(i8*)\n");
    out.push_str("declare i32 @runtime_env_has(i8*)\n");
    out.push_str("declare i32 @runtime_env_set(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_env_remove(i8*)\n");
    out.push_str("declare %LimeMap @runtime_env_all()\n\n");

    // Regex runtime declarations (Phase C-1.10)
    out.push_str("declare i8* @runtime_regex_compile(i8*)\n");
    out.push_str("declare i32 @runtime_regex_is_match(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_regex_find(i8*, i8*)\n");
    out.push_str("declare %LimeList @runtime_regex_find_all(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_regex_replace(i8*, i8*, i8*)\n");
    out.push_str("declare i8* @runtime_regex_replace_all(i8*, i8*, i8*)\n");
    out.push_str("declare %LimeList @runtime_regex_split(i8*, i8*)\n\n");

    // Process runtime declarations (Phase C-1.11)
    out.push_str("declare i64 @runtime_process_spawn(i8*, %LimeList)\n");
    out.push_str("declare i8* @runtime_process_run(i8*, %LimeList)\n");
    out.push_str("declare i8* @runtime_process_output(i8*, %LimeList)\n");
    out.push_str("declare i64 @runtime_process_wait(i64)\n");
    out.push_str("declare i32 @runtime_process_kill(i64)\n");
    out.push_str("declare i8* @runtime_process_status(i64)\n");
    out.push_str("declare %LimeList @runtime_process_args()\n\n");

    // Requests runtime declarations (Phase C-1.12)
    out.push_str("declare i8* @runtime_requests_client_new()\n");
    out.push_str("declare i8* @runtime_requests_client_builder_new()\n");
    out.push_str("declare i8* @runtime_requests_client_builder_build(i8*)\n");
    out.push_str("declare void @runtime_requests_client_builder_default_headers(i8*, i8*)\n");
    out.push_str("declare void @runtime_requests_client_builder_timeout(i8*, i64)\n");
    out.push_str("declare void @runtime_requests_client_builder_redirect_limit(i8*, i64)\n");
    out.push_str("declare void @runtime_requests_client_builder_redirect_disabled(i8*)\n");
    out.push_str("declare void @runtime_requests_client_builder_proxy(i8*, i8*)\n");
    out.push_str("declare void @runtime_requests_client_builder_tls_config(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_requests_request_builder_new(i8*, i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_header(i8*, i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_headers(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_query(i8*, %LimeList)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_body_bytes(i8*, i8*, i64)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_body_str(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_json(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_form(i8*, %LimeList)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_multipart(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_timeout(i8*, i64)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_redirect_limit(i8*, i64)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_redirect_disabled(i8*)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_basic_auth(i8*, i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_bearer_auth(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_requests_send(i8*)\n");
    out.push_str("declare i64 @runtime_requests_response_status(i8*)\n");
    out.push_str("declare i8* @runtime_requests_response_headers(i8*)\n");
    out.push_str("declare i8* @runtime_requests_response_url(i8*)\n");
    out.push_str("declare i8* @runtime_requests_response_text(i8*)\n");
    out.push_str("declare i8* @runtime_requests_response_bytes(i8*, i64*)\n");
    out.push_str("declare i8* @runtime_requests_response_json(i8*)\n");
    out.push_str("declare i64 @runtime_requests_response_content_length(i8*)\n");
    out.push_str("declare i32 @runtime_requests_response_is_success(i8*)\n");
    out.push_str("declare i32 @runtime_requests_response_is_client_error(i8*)\n");
    out.push_str("declare i32 @runtime_requests_response_is_server_error(i8*)\n");
    out.push_str("declare i8* @runtime_requests_response_error_for_status(i8*)\n");
    out.push_str("declare i64 @runtime_requests_status_code_code(i64)\n");
    out.push_str("declare i32 @runtime_requests_status_code_is_success(i64)\n");
    out.push_str("declare i32 @runtime_requests_status_code_is_client_error(i64)\n");
    out.push_str("declare i32 @runtime_requests_status_code_is_server_error(i64)\n");
    out.push_str("declare i32 @runtime_requests_status_code_is_redirect(i64)\n");
    out.push_str("declare i8* @runtime_requests_header_map_new()\n");
    out.push_str("declare i32 @runtime_requests_header_map_insert(i8*, i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_header_map_append(i8*, i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_header_map_remove(i8*, i8*, i8*)\n");
    out.push_str("declare i8* @runtime_requests_header_map_get(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_header_map_contains(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_requests_multipart_new()\n");
    out.push_str("declare i32 @runtime_requests_multipart_text(i8*, i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_multipart_file(i8*, i8*, i8*)\n");
    out.push_str(
        "declare i32 @runtime_requests_multipart_file_with_metadata(i8*, i8*, i8*, i8*, i8*)\n",
    );
    out.push_str("declare i8* @runtime_requests_tls_config_new()\n");
    out.push_str("declare i32 @runtime_requests_tls_config_add_ca_cert(i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_tls_config_add_client_cert(i8*, i8*, i8*)\n");
    out.push_str("declare i32 @runtime_requests_tls_config_danger_accept_invalid_certs(i8*)\n");
    out.push_str("declare i32 @runtime_requests_tls_config_danger_accept_invalid_hostnames(i8*)\n");
    out.push_str("declare i8* @runtime_requests_cookie_jar_new()\n");
    out.push_str("declare i32 @runtime_requests_cookie_jar_add(i8*, i8*)\n");
    out.push_str("declare i8* @runtime_requests_cookie_parse(i8*)\n");
    out.push_str("declare i64 @runtime_requests_response_copy_to(i8*, i8*)\n");
    out.push_str("declare %LimeList @runtime_requests_response_chunks(i8*, i64)\n");
    out.push_str("declare i8* @runtime_requests_response_stream(i8*)\n");
    out.push_str("declare i8* @runtime_requests_stream_read(i8*, i64, i64*)\n");
    out.push_str("declare i32 @runtime_requests_stream_has_more(i8*)\n");
    out.push_str("declare void @runtime_requests_client_free(i8*)\n");
    out.push_str("declare void @runtime_requests_request_builder_free(i8*)\n");
    out.push_str("declare void @runtime_requests_response_free(i8*)\n");
    out.push_str("declare void @runtime_requests_header_map_free(i8*)\n");
    out.push_str("declare void @runtime_requests_multipart_free(i8*)\n");
    out.push_str("declare void @runtime_requests_tls_config_free(i8*)\n");
    out.push_str("declare void @runtime_requests_cookie_jar_free(i8*)\n");
    out.push_str("declare void @runtime_requests_stream_free(i8*)\n");
    // Session
    out.push_str("declare i8* @runtime_requests_session_new()\n");
    out.push_str("declare i8* @runtime_requests_session_request(i8*, i8*, i8*)\n");
    out.push_str("declare void @runtime_requests_session_free(i8*)\n");
    // New builder methods
    out.push_str("declare i32 @runtime_requests_request_builder_set_headers(i8*, %LimeList)\n");
    out.push_str("declare i32 @runtime_requests_request_builder_verify(i8*, i32)\n");
    // Response headers as list
    out.push_str("declare %LimeList @runtime_requests_response_headers_list(i8*)\n");
    // Cookie jar extended operations
    out.push_str("declare i32 @runtime_requests_cookie_jar_add_parsed(i8*, i8*)\n");
    out.push_str("declare void @runtime_requests_cookie_jar_update_from_response(i8*, i8*, i8*)\n");
    out.push_str("declare i8* @runtime_requests_cookie_jar_get_cookie_header(i8*, i8*)\n");
    out.push_str("declare %LimeList @runtime_requests_cookie_jar_get_all(i8*)\n");
    out.push_str("declare i8* @runtime_requests_cookie_jar_get(i8*, i8*)\n");
    // Session setters
    out.push_str("declare i32 @runtime_requests_session_set_default_headers(i8*, %LimeList)\n");
    out.push_str("declare i32 @runtime_requests_session_set_default_params(i8*, %LimeList)\n");
    out.push_str("declare i32 @runtime_requests_session_set_timeout(i8*, i64)\n");
    out.push_str("declare i32 @runtime_requests_session_set_verify(i8*, i32)\n");
    out.push_str("declare i32 @runtime_requests_session_set_redirect_limit(i8*, i64)\n");
    out.push_str("declare i32 @runtime_requests_session_set_disable_redirects(i8*, i32)\n");
    out.push_str("declare %LimeList @runtime_requests_session_cookies(i8*)\n");
    // Redirect history
    out.push_str("declare i8* @runtime_requests_redirect_history_new()\n");
    out.push_str("declare void @runtime_requests_redirect_history_add(i8*, i64, i8*, i8*)\n");
    out.push_str("declare %LimeList @runtime_requests_redirect_history_list(i8*)\n");
    out.push_str("declare void @runtime_requests_redirect_history_free(i8*)\n");
    // Response redirect history
    out.push_str("declare %LimeList @runtime_requests_response_redirect_history(i8*)\n\n");

    // Format strings for print/println builtin lowering (Phase 2)
    out.push_str("@.str.int   = private unnamed_addr constant [5 x i8] c\"%lld\\00\"\n");
    out.push_str("@.str.int_nl = private unnamed_addr constant [6 x i8] c\"%lld\\0A\\00\"\n");
    out.push_str("@.str.float  = private unnamed_addr constant [6 x i8] c\"%.16g\\00\"\n");
    out.push_str("@.str.float_nl = private unnamed_addr constant [7 x i8] c\"%.16g\\0A\\00\"\n");
    out.push_str("@.str.str    = private unnamed_addr constant [3 x i8] c\"%s\\00\"\n");
    out.push_str("@.str.str_nl = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\"\n");
    out.push_str("@.str.true   = private unnamed_addr constant [5 x i8] c\"true\\00\"\n");
    out.push_str("@.str.false  = private unnamed_addr constant [6 x i8] c\"false\\00\"\n");
    out.push_str("@.str.panic_msg = private unnamed_addr constant [31 x i8] c\"extract() called on None/Error\\00\"\n\n");

    // Aggregate type declarations
    emit_aggregate_decls(&mut out, defs);

    // Charger FFI: declare native extern symbols so the linker can resolve
    // them. Signatures use sret/byval to match the Windows x64 MSVC ABI.
    emit_extern_declarations(&mut out, defs);

    // Phase 5: collect string literals and emit globals

    let mut string_literals = collect_string_literals(defs);
    // Iteration 34 PH4: register enum variant names as string globals so
    // println(enum_value) can print the variant name natively.
    for (_base, variants) in &defs.states {
        for v in variants {
            string_literals
                .entry(v.clone())
                .or_insert_with(|| format!(".str.var.{}", v));
        }
    }
    for (s, name) in &string_literals {
        let len = s.len() + 1;
        let escaped = escape_llvm_string(s);
        out.push_str(&format!(
            "@{} = private unnamed_addr constant [{} x i8] c\"{}\\00\"\n",
            name, len, escaped
        ));
    }
    if !string_literals.is_empty() {
        out.push('\n');
    }

    // Phase 6: Generic monomorphization - collect generic calls and create monomorphized copies
    let (mono_name_map, mono_fdefs) = monomorphize_all(defs);

    // Emit monomorphized function definitions first
    for (mangled, mono_fdef) in &mono_fdefs {
        let (func_ir, fw) = fn_builder::codegen_function(
            defs,
            memory,
            &string_literals,
            &mono_name_map,
            &mono_fdefs,
            mangled,
            mono_fdef,
        );
        out.push_str(&func_ir);
        warnings.extend(fw);
    }

    // Function definitions using fn_builder (Phase 1) - skip generic templates
    for (name, fdef) in &defs.functions {
        if !fdef.type_params.is_empty() {
            continue;
        }
        let (func_ir, fw) = fn_builder::codegen_function(
            defs,
            memory,
            &string_literals,
            &mono_name_map,
            &mono_fdefs,
            name,
            fdef,
        );
        out.push_str(&func_ir);
        warnings.extend(fw);
    }

    // Phase 7: Struct method function definitions
    for (sname, sdef) in &defs.structs {
        for (mname, mdef) in &sdef.methods {
            if !mdef.type_params.is_empty() {
                continue;
            }
            let method_func_name = format!("{}_{}", sname, mname);
            // Prepend self parameter (struct type name, not LLVM IR notation)
            let mut params = vec![(String::from("self"), sname.clone())];
            params.extend(mdef.params.clone());
            let method_fdef = FunctionDef {
                type_params: Vec::new(),
                constraints: Vec::new(),
                params,
                return_type: mdef.return_type.clone(),
                body: mdef.body.clone(),
                is_async: false,
            };
            let (func_ir, fw) = fn_builder::codegen_function(
                defs,
                memory,
                &string_literals,
                &mono_name_map,
                &mono_fdefs,
                &method_func_name,
                &method_fdef,
            );
            out.push_str(&func_ir);
            warnings.extend(fw);
        }
    }

    // Optimization attributes referenced by `local_unnamed_addr #0` on every
    // user function. `nounwind` lets LLVM hoist loop-invariant code and apply
    // aggressive loop optimizations that it would otherwise inhibit.
    out.push_str("\nattributes #0 = { nounwind willreturn alwaysinline }\n");

    // C runtime main wrapper
    emit_main_wrapper(&mut out, defs);

    (out, warnings)
}

/// Phase 5: escape a string for LLVM IR constant format.
fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            0 => out.push_str("\\00"),
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\r' => out.push_str("\\0D"),
            b'\t' => out.push_str("\\09"),
            0x20..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("\\{:02X}", b)),
        }
    }
    out
}

/// Phase 5: pre-scan all function bodies for Expr::StringLit, collect unique strings.
fn collect_string_literals(defs: &Defs) -> HashMap<String, String> {
    let mut strings = HashMap::new();
    let mut idx = 0usize;
    for (_, fdef) in &defs.functions {
        collect_strings_from_stmts(&fdef.body, &mut strings, &mut idx);
    }
    // Phase 7: also scan struct method bodies
    for (_, sdef) in &defs.structs {
        for (_, mdef) in &sdef.methods {
            collect_strings_from_stmts(&mdef.body, &mut strings, &mut idx);
        }
    }
    strings
}

fn collect_strings_from_stmts(
    stmts: &[Stmt],
    strings: &mut HashMap<String, String>,
    idx: &mut usize,
) {
    for s in stmts {
        match s {
            Stmt::Let { value, .. } => collect_strings_from_expr(value, strings, idx),
            Stmt::Return {
                explicit_type: _,
                value,
            } => {
                if let Some(e) = value {
                    collect_strings_from_expr(e, strings, idx);
                }
            }
            Stmt::Expr(e) => collect_strings_from_expr(e, strings, idx),
            Stmt::Assign { value, .. } => collect_strings_from_expr(value, strings, idx),
            Stmt::If {
                cond,
                then_branch,
                else_branch,
            } => {
                collect_strings_from_expr(cond, strings, idx);
                collect_strings_from_stmts(then_branch, strings, idx);
                if let Some(eb) = else_branch {
                    collect_strings_from_stmts(eb, strings, idx);
                }
            }
            Stmt::While { cond, body } => {
                collect_strings_from_expr(cond, strings, idx);
                collect_strings_from_stmts(body, strings, idx);
            }
            Stmt::For { iterable, body, .. } => {
                collect_strings_from_expr(iterable, strings, idx);
                collect_strings_from_stmts(body, strings, idx);
            }
            Stmt::Match { expr, arms } => {
                collect_strings_from_expr(expr, strings, idx);
                for (_, body) in arms {
                    collect_strings_from_stmts(body, strings, idx);
                }
            }
            _ => {}
        }
    }
}

fn collect_strings_from_expr(e: &Expr, strings: &mut HashMap<String, String>, idx: &mut usize) {
    match e {
        Expr::StringLit(s) => {
            if !strings.contains_key(s) {
                let name = format!(".str.{}", *idx);
                *idx += 1;
                strings.insert(s.clone(), name);
            }
        }
        Expr::BinOp { left, right, .. } => {
            collect_strings_from_expr(left, strings, idx);
            collect_strings_from_expr(right, strings, idx);
        }
        Expr::UnOp { operand, .. } => collect_strings_from_expr(operand, strings, idx),
        Expr::Call { args, .. } => {
            for a in args {
                collect_strings_from_expr(a, strings, idx);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_strings_from_expr(object, strings, idx);
            for a in args {
                collect_strings_from_expr(a, strings, idx);
            }
        }
        Expr::FieldAccess { object, .. } => collect_strings_from_expr(object, strings, idx),
        Expr::Array(items) => {
            for it in items {
                collect_strings_from_expr(it, strings, idx);
            }
        }
        Expr::Range { start, end } => {
            collect_strings_from_expr(start, strings, idx);
            collect_strings_from_expr(end, strings, idx);
        }
        Expr::Await(inner) => collect_strings_from_expr(inner, strings, idx),
        _ => {}
    }
}

// Phase 6: Generic monomorphization.
// Parse "func(i64, str)" into ("func", ["i64", "str"])
fn parse_generic_call_name(func: &str) -> Option<(&str, Vec<&str>)> {
    if let Some(paren_idx) = func.find('(') {
        let base = &func[..paren_idx];
        let inner = &func[paren_idx + 1..func.len() - 1];
        let args: Vec<&str> = if inner.is_empty() {
            Vec::new()
        } else {
            inner.split(", ").collect()
        };
        Some((base, args))
    } else {
        None
    }
}

fn monomorphize_type_str(t: &str, type_params: &[String], type_args: &[&str]) -> String {
    let mut result = t.to_string();
    for (i, tp) in type_params.iter().enumerate() {
        if i < type_args.len() {
            result = result.replace(tp.as_str(), type_args[i]);
        }
    }
    result
}

fn monomorphize_function(
    fdef: &FunctionDef,
    type_params: &[String],
    type_args: &[&str],
) -> FunctionDef {
    let params: Vec<(String, String)> = fdef
        .params
        .iter()
        .map(|(n, t)| (n.clone(), monomorphize_type_str(t, type_params, type_args)))
        .collect();
    let return_type = fdef
        .return_type
        .as_ref()
        .map(|rt| monomorphize_type_str(rt, type_params, type_args));
    FunctionDef {
        type_params: Vec::new(),
        constraints: Vec::new(),
        params,
        return_type,
        body: fdef.body.clone(),
        is_async: fdef.is_async,
    }
}

// Scan all function bodies for generic calls and monomorphize them.
fn monomorphize_all(defs: &Defs) -> (HashMap<String, String>, HashMap<String, FunctionDef>) {
    let mut mono_name_map: HashMap<String, String> = HashMap::new();
    let mut mono_fdefs: HashMap<String, FunctionDef> = HashMap::new();

    let mut worklist: Vec<String> = defs.functions.keys().cloned().collect();
    let mut done: HashMap<String, bool> = HashMap::new();

    while let Some(name) = worklist.pop() {
        if done.contains_key(&name) {
            continue;
        }
        done.insert(name.clone(), true);

        let fdef = match defs.functions.get(&name) {
            Some(f) => f,
            None => continue,
        };

        collect_mono_from_stmts(
            &fdef.body,
            defs,
            &mut mono_name_map,
            &mut mono_fdefs,
            &mut worklist,
        );
    }

    (mono_name_map, mono_fdefs)
}

fn collect_mono_from_stmts(
    stmts: &[Stmt],
    defs: &Defs,
    mono_name_map: &mut HashMap<String, String>,
    mono_fdefs: &mut HashMap<String, FunctionDef>,
    worklist: &mut Vec<String>,
) {
    for s in stmts {
        match s {
            Stmt::Let { value, .. } => {
                collect_mono_from_expr(value, defs, mono_name_map, mono_fdefs, worklist)
            }
            Stmt::Return {
                explicit_type: _,
                value,
            } => {
                if let Some(e) = value {
                    collect_mono_from_expr(e, defs, mono_name_map, mono_fdefs, worklist);
                }
            }
            Stmt::Expr(e) => collect_mono_from_expr(e, defs, mono_name_map, mono_fdefs, worklist),
            Stmt::Assign { value, .. } => {
                collect_mono_from_expr(value, defs, mono_name_map, mono_fdefs, worklist)
            }
            Stmt::If {
                cond,
                then_branch,
                else_branch,
            } => {
                collect_mono_from_expr(cond, defs, mono_name_map, mono_fdefs, worklist);
                collect_mono_from_stmts(then_branch, defs, mono_name_map, mono_fdefs, worklist);
                if let Some(eb) = else_branch {
                    collect_mono_from_stmts(eb, defs, mono_name_map, mono_fdefs, worklist);
                }
            }
            Stmt::While { cond, body } => {
                collect_mono_from_expr(cond, defs, mono_name_map, mono_fdefs, worklist);
                collect_mono_from_stmts(body, defs, mono_name_map, mono_fdefs, worklist);
            }
            Stmt::For { iterable, body, .. } => {
                collect_mono_from_expr(iterable, defs, mono_name_map, mono_fdefs, worklist);
                collect_mono_from_stmts(body, defs, mono_name_map, mono_fdefs, worklist);
            }
            Stmt::Match { expr, arms } => {
                collect_mono_from_expr(expr, defs, mono_name_map, mono_fdefs, worklist);
                for (_, body) in arms {
                    collect_mono_from_stmts(body, defs, mono_name_map, mono_fdefs, worklist);
                }
            }
            _ => {}
        }
    }
}

fn collect_mono_from_expr(
    e: &Expr,
    defs: &Defs,
    mono_name_map: &mut HashMap<String, String>,
    mono_fdefs: &mut HashMap<String, FunctionDef>,
    worklist: &mut Vec<String>,
) {
    match e {
        Expr::Call { func, args } => {
            // Check if this is a generic call like "max(i64)"
            if let Some((base, type_strs)) = parse_generic_call_name(func) {
                if let Some(fdef) = defs.functions.get(base) {
                    if !fdef.type_params.is_empty() {
                        let call_name = func.clone();
                        if !mono_name_map.contains_key(&call_name) {
                            let mangled = crate::mangled_name(base, &type_strs);
                            let type_params: Vec<&str> = type_strs.iter().map(|s| *s).collect();
                            let mono = monomorphize_function(fdef, &fdef.type_params, &type_params);
                            mono_name_map.insert(call_name.clone(), mangled.clone());
                            mono_fdefs.insert(mangled.clone(), mono);
                            worklist.push(mangled);
                        }
                    }
                }
            }
            for a in args {
                collect_mono_from_expr(a, defs, mono_name_map, mono_fdefs, worklist);
            }
        }
        Expr::BinOp { left, right, .. } => {
            collect_mono_from_expr(left, defs, mono_name_map, mono_fdefs, worklist);
            collect_mono_from_expr(right, defs, mono_name_map, mono_fdefs, worklist);
        }
        Expr::UnOp { operand, .. } => {
            collect_mono_from_expr(operand, defs, mono_name_map, mono_fdefs, worklist)
        }
        Expr::MethodCall { object, args, .. } => {
            collect_mono_from_expr(object, defs, mono_name_map, mono_fdefs, worklist);
            for a in args {
                collect_mono_from_expr(a, defs, mono_name_map, mono_fdefs, worklist);
            }
        }
        Expr::FieldAccess { object, .. } => {
            collect_mono_from_expr(object, defs, mono_name_map, mono_fdefs, worklist)
        }
        Expr::Array(items) => {
            for it in items {
                collect_mono_from_expr(it, defs, mono_name_map, mono_fdefs, worklist);
            }
        }
        Expr::Range { start, end } => {
            collect_mono_from_expr(start, defs, mono_name_map, mono_fdefs, worklist);
            collect_mono_from_expr(end, defs, mono_name_map, mono_fdefs, worklist);
        }
        Expr::Await(inner) => {
            collect_mono_from_expr(inner, defs, mono_name_map, mono_fdefs, worklist)
        }
        _ => {}
    }
}

/// 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｪ縺ｿ縺ｪ縺・定義: struct / state / list / option / interface
fn emit_aggregate_decls(out: &mut String, defs: &Defs) {
    // struct declarations with real field types (Phase 3)
    for (sname, sdef) in &defs.structs {
        let field_tys: Vec<String> = sdef
            .fields
            .iter()
            .map(|(_, ftype)| llvm_type_name(&type_from_str(ftype, defs)))
            .collect();
        out.push_str(&format!(
            "%{} = type {{ {} }}\n",
            sname,
            field_tys.join(", ")
        ));
    }
    // state declarations as tagged unions (Phase 4)
    for sname in defs.states.keys() {
        out.push_str(&format!("%{} = type {{ i32, [4 x i64] }}\n", sname));
    }
    out.push('\n');
}

/// Declare Charger FFI extern symbols at the LLVM module level so calls to
/// them verify and link. Struct returns use `sret` (or a plain `this` pointer
/// for C++ constructors/destructors); struct arguments use `byval`.
fn emit_extern_declarations(out: &mut String, defs: &Defs) {
    for ((_name, _arity), (symbol, params, ret)) in &defs.extern_symbols {
        let rt = extern_ret_type(ret.as_deref());
        let is_this_return = symbol.starts_with("??0") || symbol.starts_with("??1");

        let mut param_decls: Vec<String> = Vec::new();
        for (first, second) in params {
            // `params` is stored as (name, type); robustly extract the *type*
            // element: it is either a known scalar keyword (Int/Float/String/
            // Bool/Unit/Void) or a struct name. The name element is never a
            // type keyword, so the type is whichever element is not a plain
            // identifier that equals the parameter name. Simplest robust rule:
            // pick the element that is a known scalar keyword, else the element
            // that is NOT the parameter name (struct types are not valid names
            // in the scalar set, so the type is the non-name element).
            let ptype = if matches!(
                first.as_str(),
                "Int" | "Float" | "String" | "Bool" | "Unit" | "Void"
            ) {
                first.clone()
            } else if matches!(
                second.as_str(),
                "Int" | "Float" | "String" | "Bool" | "Unit" | "Void"
            ) {
                second.clone()
            } else if first == "Point" || first == "Widget" || first.starts_with("__") {
                // first is the struct type name (type, name) ordering
                first.clone()
            } else {
                // default: second element is the type (name, type) ordering
                second.clone()
            };
            let pt = extern_param_type(&ptype);
            if matches!(pt, Type::Struct(_)) {
                param_decls.push(format!("ptr byval({})", llvm_type_name(&pt)));
            } else {
                param_decls.push(llvm_type_name(&pt));
            }
        }

        if matches!(rt, Type::Struct(_)) {
            if is_this_return {
                let mut decls = vec!["ptr".to_string()];
                decls.extend(param_decls);
                out.push_str(&format!(
                    "declare void @\"{}\"({})\n",
                    symbol,
                    decls.join(", ")
                ));
            } else {
                let mut decls = vec![format!("ptr sret({})", llvm_type_name(&rt))];
                decls.extend(param_decls);
                out.push_str(&format!(
                    "declare void @\"{}\"({})\n",
                    symbol,
                    decls.join(", ")
                ));
            }
        } else {
            out.push_str(&format!(
                "declare {} @\"{}\"({})\n",
                llvm_type_name(&rt),
                symbol,
                param_decls.join(", ")
            ));
        }
    }
    if !defs.extern_symbols.is_empty() {
        out.push('\n');
    }
}

fn extern_ret_type(s: Option<&str>) -> Type {
    match s {
        Some("Int") => Type::Int,
        Some("Float") => Type::Float,
        Some("String") => Type::String,
        Some("Bool") => Type::Bool,
        Some("Unit") | Some("Void") | None => Type::Unit,
        // Task #2: opaque C pointer handle -> `ptr` return slot (not sret).
        Some(other) if other.starts_with("opaque(") => crate::extern_opaque_type(other),
        Some(other) => Type::Struct(other.to_string()),
    }
}

fn extern_param_type(s: &str) -> Type {
    // Task #1: a C function-pointer parameter (`int (*)(int, int)`) is surfaced
    // to Lime as a `fn(Int, Int) -> Int` type. Parse it into `Type::Fn` so the
    // extern declaration encodes it as a raw function pointer (i8*) instead of a
    // struct.
    if let Some(rest) = s.strip_prefix("fn(") {
        if let Some(arrow) = rest.find(") -> ") {
            let params_str = &rest[..arrow];
            let ret_str = &rest[arrow + 5..];
            let param_types: Vec<Type> = if params_str.trim().is_empty() {
                Vec::new()
            } else {
                params_str
                    .split(',')
                    .map(|p| extern_param_type(p.trim()))
                    .collect()
            };
            let ret_type = extern_param_type(ret_str.trim());
            return Type::Fn(param_types, Box::new(ret_type));
        }
        // The opaque shorthand `fn(...)` is what `parse_type` produces for the
        // Charger-emitted `Callback` type (Task #1). Without this arm it fell
        // through to `Type::Struct("fn(...)")` and the declaration slot became
        // `ptr byval(%fn)` — an undefined LLVM type that blocked object
        // emission for any callback-taking extern.
        if rest.trim() == "...)" || rest.trim() == "..." {
            return Type::Fn(Vec::new(), Box::new(Type::Unit));
        }
    }
    // Task #2: an opaque C pointer handle parameter (`struct X*` / `void*`) is
    // declared as a bare `ptr` slot, not a struct (which would be `byval`).
    if s.starts_with("opaque(") {
        return crate::extern_opaque_type(s);
    }
    match s {
        "Int" => Type::Int,
        "Float" => Type::Float,
        "String" => Type::String,
        "Bool" => Type::Bool,
        "Unit" | "Void" => Type::Unit,
        other => Type::Struct(other.to_string()),
    }
}

fn emit_main_wrapper(out: &mut String, defs: &Defs) {
    if !defs.functions.contains_key("main") {
        return;
    }
    out.push_str("define i32 @main() {\n");
    out.push_str(&format!("  call void @{}()\n", LIME_MAIN));
    out.push_str("  ret i32 0\n");
    out.push_str("}\n\n");
}
