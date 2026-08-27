#ifndef LIBPTRTYPEDEF_H
#define LIBPTRTYPEDEF_H

#ifdef __cplusplus
extern "C" {
#endif

/* Iteration 29 regression fixture.
 *
 * Pointer typedefs whose POINTEE is itself a scalar typedef
 * (`typedef unsigned char byte_t; typedef byte_t *bytep;`) exposed a
 * Charger bug: the post-walk SCALAR_TYPEDEFS rebuild published self-loop
 * entries (`byte_t -> "byte_t"`) and the pointer-alias pass fed that loop
 * member into parse_c_type, recursing forever -> main-thread stack overflow
 * (measured on libpng `typedef png_byte * png_bytep;` and libjpeg-turbo
 * `JSAMPROW`-style typedefs). This fixture keeps that exact shape under
 * permanent gate coverage, alongside struct-pointee and const variants.
 */

typedef unsigned char byte_t;
typedef byte_t *bytep;
typedef const byte_t *cbytep;

typedef struct PTObj_ {
    int v;
} *PTObj;

bytep ptd_make(byte_t init);
byte_t ptd_get(const bytep p);
void   ptd_set(bytep p, byte_t v);
int    ptd_sum(cbytep p, int n);
PTObj  ptd_obj_make(int v);
int    ptd_obj_get(PTObj o);

/* Link anchor so the prepared artifact links even if a smoke references only
 * generated shims. Generic convention shared by probe corpora. */
int ptd_link_anchor(void);

#ifdef __cplusplus
}
#endif

#endif
