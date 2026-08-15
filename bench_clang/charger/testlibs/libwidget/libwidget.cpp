#include "libwidget.h"
#include <stdlib.h>

long long Widget::count = 0;

Widget::Widget(long long x, long long y) : x_(x), y_(y) {
    count++;
}

long long Widget::area() const {
    return x_ * y_;
}

long long Widget::area(long long scale) const {
    return x_ * y_ * scale;
}

long long Widget::get_x() const {
    return x_;
}

// Pointer / object-lifetime API (Slice #3).
Widget* widget_new(long long x, long long y) {
    Widget* w = (Widget*)malloc(sizeof(Widget));
    // Construct in place (no real C++ ctor call needed for POD-like layout,
    // but we emulate it to keep x_/y_ initialized).
    w->x_ = x;
    w->y_ = y;
    return w;
}

void widget_move(Widget* w, long long dx, long long dy) {
    w->x_ = w->x_ + dx;
    w->y_ = w->y_ + dy;
}

long long widget_area(Widget* w) {
    return w->x_ * w->y_;
}

long long widget_get_x(Widget* w) {
    return w->x_;
}

void widget_free(Widget* w) {
    free(w);
}
