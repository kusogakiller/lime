#include "libshape.h"
#include <stdlib.h>

Shape::~Shape() {}

long long Shape::area() const {
    // Base stub — must NOT be called when the object is a Circle.
    return -1;
}

Circle::Circle(long long r) : r_(r) {}

long long Circle::area() const {
    // Derived implementation: area = r * r (simplified, no floating point).
    return r_ * r_;
}

Shape* make_circle(long long r) {
    return new Circle(r);
}

long long shape_area(Shape* s) {
    // Virtual dispatch through the C++ vtable: resolves to Circle::area.
    return s->area();
}

void shape_free(Shape* s) {
    delete s;
}
