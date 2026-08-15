#ifndef LIBWIDGET_H
#define LIBWIDGET_H

// NOTE: fields/params use `long long` so the C++ class layout matches Lime's
// `Int` (which lowers to LLVM i64). This keeps the vertical-slice ABI check
// honest: Lime `struct Widget { x_: Int, y_: Int }` is 16 bytes (2xi64) and the
// C++ `Widget` must be the same size/field-order for by-value params/returns.
//
// Slice #1 (f953a5c): stack/by-value API (`Widget` returned by value).
// Slice #3 (object lifetime): pointer API — `Widget*` is an opaque handle on
// the Lime side, and the Lime program must observe mutation through the
// pointer across new / method / destructor.
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

// Pointer / object-lifetime API (Slice #3).
// Heap-allocate a Widget and return its pointer (opaque handle on the Lime side).
Widget* widget_new(long long x, long long y);

// Mutate the pointed-to Widget (pointer-based method call).
void widget_move(Widget* w, long long dx, long long dy);

// Read the pointed-to Widget's area (pointer-based method call).
long long widget_area(Widget* w);

// Read the pointed-to Widget's x field (pointer-based getter).
long long widget_get_x(Widget* w);

// Destroy the Widget (explicit destructor / free).
void widget_free(Widget* w);

#endif
