#include "libstack.h"
#include <stdlib.h>

Stack<long long>* make_stack() {
    return new Stack<long long>();
}

void stack_push(Stack<long long>* s, long long v) {
    s->push(v);
}

long long stack_size(Stack<long long>* s) {
    return s->size();
}

void stack_free(Stack<long long>* s) {
    delete s;
}
