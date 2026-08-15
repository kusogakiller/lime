#ifndef LIBSHAPE_H
#define LIBSHAPE_H

// Inheritance / virtual dispatch vertical-slice library for Charger #5.
// `Shape` is an abstract base with a virtual method; `Circle` overrides it.
// The Lime side holds a `Shape*` as an opaque handle and calls a free function
// `shape_area` that internally dispatches through the C++ vtable, proving the
// derived implementation is actually invoked (not the base stub).

class Shape {
public:
    virtual long long area() const;
    virtual ~Shape();
};

class Circle : public Shape {
public:
    explicit Circle(long long r);
    long long area() const override;
private:
    long long r_;
};

// Factory: returns a Base* (Shape*) to a Derived (Circle) instance.
Shape* make_circle(long long r);

// Dispatches through the C++ vtable: calls s->area(), which resolves to
// Circle::area at runtime.
long long shape_area(Shape* s);

// Destroys via the virtual destructor.
void shape_free(Shape* s);

#endif
