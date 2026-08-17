/* Minimal jconfig.h synthesized for Charger single-header API extraction.
 * Derived from jconfig.h.in (CMake template) for libjpeg-turbo 3.1.0. */
#include <stdio.h>   /* jpeglib.h references FILE (jpeg_stdio_*) */
#define JPEG_LIB_VERSION  90
#define LIBJPEG_TURBO_VERSION  "3.1.0"
#define LIBJPEG_TURBO_VERSION_NUMBER  3001000

#define MEM_SRCDST_SUPPORTED  1

#ifdef _WIN32
#undef RIGHT_SHIFT_IS_UNSIGNED
#ifndef __RPCNDR_H__
typedef unsigned char boolean;
#endif
#define HAVE_BOOLEAN
#if !(defined(_BASETSD_H_) || defined(_BASETSD_H))
typedef short INT16;
typedef signed int INT32;
#endif
#define XMD_H
#endif
