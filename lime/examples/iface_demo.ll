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

%Cat = type { i8*, i64 }
%Dog = type { i8*, i64 }
%Option = type { i32, [4 x i64] }
%Result = type { i32, [4 x i64] }

@.str.0 = private unnamed_addr constant [1 x i8] c"\00"
@.str.2 = private unnamed_addr constant [5 x i8] c"Mimi\00"
@.str.3 = private unnamed_addr constant [5 x i8] c"meow\00"
@.str.1 = private unnamed_addr constant [4 x i8] c"Rex\00"
@.str.4 = private unnamed_addr constant [5 x i8] c"woof\00"


; Function make_sound()
define void @make_sound (%LimeIface %p0) {
L0:
  %t0 = alloca %LimeIface, align 8
  store %LimeIface %p0, %LimeIface* %t0, align 8
  %t1 = load %LimeIface, %LimeIface* %t0, align 8
  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([3 x i8], ptr @.str.str, i64 0, i64 0), i8* 0)
  ret void
}

; Function main()
define void @main_lime () {
L0:
  %t0 = getelementptr inbounds [4 x i8], ptr @.str.1, i64 0, i64 0
  %t1 = insertvalue %Dog undef, i8* %t0, 0
  %t2 = insertvalue %Dog %t1, i64 4, 1
  %t3 = alloca %Dog, align 8
  store %Dog %t2, %Dog* %t3, align 8
  %t4 = load %Dog, %Dog* %t3, align 8
  %t5 = getelementptr inbounds [1 x i8], ptr @.str.0, i64 0, i64 0
  %t6 = call i8* @Dog_speak(%Dog %t4, i8* %t5)
  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([3 x i8], ptr @.str.str, i64 0, i64 0), i8* %t6)
  %t7 = load %Dog, %Dog* %t3, align 8
  %t8 = call i64 @Dog_legs(%Dog %t7)
  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], ptr @.str.int, i64 0, i64 0), i64 %t8)
  %t9 = getelementptr inbounds [5 x i8], ptr @.str.2, i64 0, i64 0
  %t10 = insertvalue %Cat undef, i8* %t9, 0
  %t11 = insertvalue %Cat %t10, i64 4, 1
  %t12 = alloca %Cat, align 8
  store %Cat %t11, %Cat* %t12, align 8
  %t13 = load %Cat, %Cat* %t12, align 8
  %t14 = getelementptr inbounds [1 x i8], ptr @.str.0, i64 0, i64 0
  %t15 = call i8* @Cat_speak(%Cat %t13, i8* %t14)
  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([3 x i8], ptr @.str.str, i64 0, i64 0), i8* %t15)
  %t16 = load %Dog, %Dog* %t3, align 8
  call void @make_sound(%LimeIface %t16)
  %t17 = load %Cat, %Cat* %t12, align 8
  call void @make_sound(%LimeIface %t17)
  ret void
}

; Function Cat_speak()
define i8* @Cat_speak (%Cat %p0, i8* %p1) {
L0:
  %t0 = alloca %Cat, align 8
  store %Cat %p0, %Cat* %t0, align 8
  %t1 = alloca i8*, align 8
  store i8* %p1, i8** %t1, align 8
  %t2 = load %Cat, %Cat* %t0, align 8
  %t3 = extractvalue %Cat %t2, 0
  %t4 = alloca i8*, align 8
  store i8* %t3, i8** %t4, align 8
  %t5 = extractvalue %Cat %t2, 1
  %t6 = alloca i64, align 8
  store i64 %t5, i64* %t6, align 8
  %t7 = getelementptr inbounds [5 x i8], ptr @.str.3, i64 0, i64 0
  ret i8* %t7
}

; Function Cat_legs()
define i64 @Cat_legs (%Cat %p0) {
L0:
  %t0 = alloca %Cat, align 8
  store %Cat %p0, %Cat* %t0, align 8
  %t1 = load %Cat, %Cat* %t0, align 8
  %t2 = extractvalue %Cat %t1, 0
  %t3 = alloca i8*, align 8
  store i8* %t2, i8** %t3, align 8
  %t4 = extractvalue %Cat %t1, 1
  %t5 = alloca i64, align 8
  store i64 %t4, i64* %t5, align 8
  ret i64 4
}

; Function Dog_legs()
define i64 @Dog_legs (%Dog %p0) {
L0:
  %t0 = alloca %Dog, align 8
  store %Dog %p0, %Dog* %t0, align 8
  %t1 = load %Dog, %Dog* %t0, align 8
  %t2 = extractvalue %Dog %t1, 0
  %t3 = alloca i8*, align 8
  store i8* %t2, i8** %t3, align 8
  %t4 = extractvalue %Dog %t1, 1
  %t5 = alloca i64, align 8
  store i64 %t4, i64* %t5, align 8
  ret i64 4
}

; Function Dog_speak()
define i8* @Dog_speak (%Dog %p0, i8* %p1) {
L0:
  %t0 = alloca %Dog, align 8
  store %Dog %p0, %Dog* %t0, align 8
  %t1 = alloca i8*, align 8
  store i8* %p1, i8** %t1, align 8
  %t2 = load %Dog, %Dog* %t0, align 8
  %t3 = extractvalue %Dog %t2, 0
  %t4 = alloca i8*, align 8
  store i8* %t3, i8** %t4, align 8
  %t5 = extractvalue %Dog %t2, 1
  %t6 = alloca i64, align 8
  store i64 %t5, i64* %t6, align 8
  %t7 = getelementptr inbounds [5 x i8], ptr @.str.4, i64 0, i64 0
  ret i8* %t7
}
define i32 @main() {
  call void @main_lime()
  ret i32 0
}

