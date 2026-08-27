#include "libubi.h"
#include <stdlib.h>

UBitView* ubi_alloc_view(void) { return (UBitView*)malloc(sizeof(UBitView)); }
void ubi_free_view(UBitView* p) { free(p); }
void ubi_set_word(UBitView* p, unsigned short v) { p->word = v; }
unsigned short ubi_get_word(UBitView* p) { return p->word; }
void ubi_set_lo(UBitView* p, unsigned int v) { p->bits.lo = v; }
unsigned int ubi_get_lo(UBitView* p) { return p->bits.lo; }
void ubi_set_hi(UBitView* p, unsigned int v) { p->bits.hi = v; }
unsigned int ubi_get_hi(UBitView* p) { return p->bits.hi; }

UBitNamed* ubi_alloc_named(void) { return (UBitNamed*)malloc(sizeof(UBitNamed)); }
void ubi_free_named(UBitNamed* p) { free(p); }
void ubi_set_named_word(UBitNamed* p, unsigned short v) { p->word = v; }
unsigned short ubi_get_named_word(UBitNamed* p) { return p->word; }

UBitHolder* ubi_alloc_holder(void) { return (UBitHolder*)malloc(sizeof(UBitHolder)); }
void ubi_free_holder(UBitHolder* p) { free(p); }
int ubi_get_holder_tag(UBitHolder* p) { return p->tag; }
void ubi_set_holder_tag(UBitHolder* p, int v) { p->tag = v; }
