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

%Option = type { i32, [4 x i64] }
%math.Result = type { i32, [4 x i64] }
%math.Option = type { i32, [4 x i64] }
%Result = type { i32, [4 x i64] }


; Function used()
define i64 @used (i64 %p0) {
L0:
  %t0 = alloca i64, align 8
  store i64 %p0, i64* %t0, align 8
  %t1 = load i64, i64* %t0, align 8
  %t2 = add i64 %t1, 1
  ret i64 %t2
}

; Function main()
define void @main_lime () {
L0:
  %t0 = call i64 @used(i64 41)
  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([6 x i8], ptr @.str.int_nl, i64 0, i64 0), i64 %t0)
  %t1 = call double @math.sqrt(double 16.0)
  %t2 = alloca double, align 8
  store double %t1, double* %t2, align 8
  %t3 = load double, double* %t2, align 8
  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], ptr @.str.float_nl, i64 0, i64 0), double %t3)
  ret void
}

; Function math.sqrt()
define double @math.sqrt (double %p0) {
L0:
  %t0 = alloca double, align 8
  store double %p0, double* %t0, align 8
}
define i32 @main() {
  call void @main_lime()
  ret i32 0
}

