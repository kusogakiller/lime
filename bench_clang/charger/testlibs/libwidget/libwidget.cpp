#include "libwidget.h"

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
