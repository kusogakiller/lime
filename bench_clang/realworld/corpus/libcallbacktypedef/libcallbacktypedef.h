#ifndef LIBCALLBACKTYPEDEF_H
#define LIBCALLBACKTYPEDEF_H

// Iteration 14: typedef function-pointer ABI hardening fixture.
typedef void (*cb_t)(int);
typedef int (*cb_ret_t)(int, void *);
typedef void (*cb_userdata_t)(int, void *);
typedef void (*cb_const_t)(const char *);
typedef void (*cb_ptr_t)(void *);

int callback_tail(cb_t cb, void *userdata);
int callback_return(cb_ret_t cb, void *userdata);
int callback_userdata(cb_userdata_t cb, void *userdata);
int callback_const(cb_const_t cb, void *userdata);
int callback_ptr(cb_ptr_t cb, void *userdata);

#endif /* LIBCALLBACKTYPEDEF_H */
