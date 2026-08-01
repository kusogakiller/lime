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
%Result = type { i32, [4 x i64] }
%Option = type { i32, [4 x i64] }


; Function add()
define i64 @add (i64 %p0, i64 %p1) {
L0:
  %t0 = alloca i64, align 8
  store i64 %p0, i64* %t0, align 8
  %t1 = alloca i64, align 8
  store i64 %p1, i64* %t1, align 8
  %t2 = load i64, i64* %t0, align 8
  %t3 = load i64, i64* %t1, align 8
  %t4 = add i64 %t2, %t3
  ret i64 %t4
}

; Function main()
define void @main_lime () {
L0:
  %t0 = insertvalue %Point undef, i64 3, 0
  %t1 = insertvalue %Point %t0, i64 4, 1
  %t2 = alloca %Point, align 8
  store %Point %t1, %Point* %t2, align 8
  %t3 = load %Point, %Point* %t2, align 8
  %t4 = extractvalue %Point %t3, 0
  %t5 = load %Point, %Point* %t2, align 8
  %t6 = extractvalue %Point %t5, 1
  %t7 = call i64 @add(i64 %t4, i64 %t6)
  %t8 = alloca i64, align 8
  store i64 %t7, i64* %t8, align 8
  %t9 = load i64, i64* %t8, align 8
  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], ptr @.str.int, i64 0, i64 0), i64 %t9)
  %t10 = alloca i64, align 8
  store i64 1, i64* %t10, align 8
  %t11 = alloca i64, align 8
  store i64 0, i64* %t11, align 8
  br label %L1
L1:
  %t12 = load i64, i64* %t10, align 8
  %t13 = icmp sle i64 %t12, 5
  br i1 %t13, label %L2, label %L3
L2:
  %t14 = load i64, i64* %t11, align 8
  %t15 = load i64, i64* %t10, align 8
  %t16 = add i64 %t14, %t15
  store i64 %t16, i64* %t11, align 8
  %t17 = load i64, i64* %t10, align 8
  %t18 = add i64 %t17, 1
  store i64 %t18, i64* %t10, align 8
  br label %L1
L3:
  %t19 = load i64, i64* %t11, align 8
  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], ptr @.str.int, i64 0, i64 0), i64 %t19)
  ret void
}
define i32 @main() {
  call void @main_lime()
  ret i32 0
}

