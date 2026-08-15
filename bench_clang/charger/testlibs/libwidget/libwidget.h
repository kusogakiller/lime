#ifndef LIBWIDGET_H
#define LIBWIDGET_H

// NOTE: fields/params use `long long` so the C++ class layout matches Lime's
// `Int` (which lowers to LLVM i64). This keeps the vertical-slice ABI check
// honest: Lime `struct Widget { x_: Int, y_: Int }` is 16 bytes (2xi64) and the
// C++ `Widget` must be the same size/field-order for by-value params/returns.
class Widget {
public:
    Widget(long long x, long long y);
    long long area() const;
    long long area(long long scale) const;
    long long get_x() const;
    static long long count;
    long long x_;
    long long y_;
};

#endif
