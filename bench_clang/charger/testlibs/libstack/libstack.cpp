#include "libstack.h"
#include <stdlib.h>

template <typename T>
Stack<T>::Stack() : top(0), cap(4) {
    data = (T*)malloc(sizeof(T) * cap);
}

template <typename T>
void Stack<T>::push(T v) {
    if (top >= cap) {
        cap *= 2;
        data = (T*)realloc(data, sizeof(T) * cap);
    }
    data[top++] = v;
}

template <typename T>
T Stack<T>::pop() {
    if (top <= 0) return T(0);
    return data[--top];
}

template <typename T>
int Stack<T>::size() const {
    return top;
}

template <typename T>
Stack<T>::~Stack() {
    free(data);
}

// Explicit instantiation for long long.
template class Stack<long long>;

Stack<long long>* make_stack() {
    return new Stack<long long>();
}

void stack_push(Stack<long long>* s, long long v) {
    s->push(v);
}

long long stack_pop(Stack<long long>* s) {
    return s->pop();
}

int stack_size(Stack<long long>* s) {
    return s->size();
}

void stack_free(Stack<long long>* s) {
    delete s;
}
