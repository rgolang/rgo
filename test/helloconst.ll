@x = private unnamed_addr constant [14 x i8] c"Hello world!\0A\00"

define i32 @main() {
entry:
	call void @unsafe.libc.puts([14 x i8]* @x, void (i64)* @main.0)
	ret i32 0
}

define void @unsafe.libc.puts(i8* %str, void (i64)* %ok) {
entry:
	%0 = call i32 @puts(i8* %str)
	%1 = zext i32 %0 to i64
	call void %ok(i64 %1)
	ret void
}

declare i32 @puts(i8* %str)

define void @main.0(i64 %code) {
entry:
	ret void
}
