; ModuleID = 'lime'
source_filename = "lime"
target triple = "x86_64-pc-windows-msvc"

%LimeList = type { i8*, i64, i64 }
%LimeOption = type { i1, i8* }
%LimeIface = type { i8*, i8* }

declare i8* @runtime_alloc(i64, i64)
declare void @runtime_print(i8*)
declare i32 @printf(i8*, ...)

declare i64 @strlen(i8*)
declare i8* @runtime_str_slice(i8*, i64, i64)
declare i8* @runtime_str_concat(i8*, i8*)
declare void @runtime_str_chars(ptr sret(%LimeList), ptr)
declare void @runtime_str_bytes(ptr sret(%LimeList), ptr)
declare void @runtime_list_add(ptr sret(%LimeList), ptr, i64)
declare void @runtime_list_set(ptr sret(%LimeList), ptr, i64, i64)

@.str.int   = private unnamed_addr constant [5 x i8] c"%lld\00"
@.str.int_nl = private unnamed_addr constant [6 x i8] c"%lld\0A\00"
@.str.float  = private unnamed_addr constant [3 x i8] c"%g\00"
@.str.float_nl = private unnamed_addr constant [4 x i8] c"%g\0A\00"
@.str.str    = private unnamed_addr constant [3 x i8] c"%s\00"
@.str.str_nl = private unnamed_addr constant [4 x i8] c"%s\0A\00"
@.str.true   = private unnamed_addr constant [5 x i8] c"true\00"
@.str.false  = private unnamed_addr constant [6 x i8] c"false\00"

%Point = type { i64, i64 }
%Option = type { i32, [4 x i64] }
%Result = type { i32, [4 x i64] }

@.str.1 = private unnamed_addr constant [7 x i8] c" world\00"
@.str.0 = private unnamed_addr constant [6 x i8] c"hello\00"


; Function make_point()
define %Point @make_point (i64 %p0, i64 %p1) {
L0:
  %t0 = alloca i64, align 8
  store i64 %p0, i64* %t0, align 8
  %t1 = alloca i64, align 8
  store i64 %p1, i64* %t1, align 8
  %t2 = load i64, i64* %t0, align 8
  %t3 = insertvalue %Point undef, i64 %t2, 0
  %t4 = load i64, i64* %t1, align 8
  %t5 = insertvalue %Point %t3, i64 %t4, 1
  ret %Point %t5
}

; Function main()
define void @main_lime () {
  ret void
}
define i32 @main() {
  call void @main_lime()
  ret i32 0
}

