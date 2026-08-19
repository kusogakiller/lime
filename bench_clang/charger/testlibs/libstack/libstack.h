#ifndef LIBSTACK_H
#define LIBSTACK_H

// Template / template-instantiation vertical-slice library for Charger #6.
// A templated `Stack<T>` is explicitly instantiated for `long long`. Charger
// must recognize the concrete instantiation `Stack<long long>` (emitted by
// clang as a concrete CXXRecordDecl) and surface it to Lime as an opaque
// handle so the FFI free functions can operate on it. Lime never sees the
// template; it only manipulates the instantiated concrete type.

template <typename T>
class Stack {
public:
    Stack();
    void push(T v);
    T pop();
    int size() const;
    ~Stack();
private:
    T* data;
    int top;
    int cap;
};

// Explicit instantiation for the slice.
extern template class Stack<long long>;

// C-compatible FFI surface (free functions, mangled by the C++ ABI).
Stack<long long>* make_stack();
void stack_push(Stack<long long>* s, long long v);
long long stack_pop(Stack<long long>* s);
int stack_size(Stack<long long>* s);
void stack_free(Stack<long long>* s);

#endif
