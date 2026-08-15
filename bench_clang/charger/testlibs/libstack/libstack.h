#ifndef LIBSTACK_H
#define LIBSTACK_H

// Template / template-instantiation vertical-slice library for Charger #6.
// `Stack<T>` is a class template; the Lime side only ever touches the
// concrete instantiation `Stack<long long>` (which Clang emits as a real
// CXXRecordDecl with mangled symbols). The Lime side holds a
// `Stack<long long>*` as an opaque handle and drives it through free
// functions, proving template instantiation is reachable from Lime.

template <typename T>
class Stack {
public:
    Stack() : data_(nullptr), sz_(0), cap_(0) {}
    ~Stack() { delete[] data_; }

    void push(T v) {
        if (sz_ == cap_) {
            cap_ = cap_ == 0 ? 4 : cap_ * 2;
            T* n = new T[cap_];
            for (long long i = 0; i < sz_; i++) n[i] = data_[i];
            delete[] data_;
            data_ = n;
        }
        data_[sz_] = v;
        sz_++;
    }

    T pop() {
        sz_--;
        return data_[sz_];
    }

    long long size() const { return sz_; }

private:
    T* data_;
    long long sz_;
    long long cap_;
};

// Concrete instantiation factory + free-function API (mangled for the
// `Stack<long long>` instantiation).
Stack<long long>* make_stack();
void stack_push(Stack<long long>* s, long long v);
long long stack_size(Stack<long long>* s);
void stack_free(Stack<long long>* s);

#endif
