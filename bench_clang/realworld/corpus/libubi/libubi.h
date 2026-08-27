#ifndef LIBUBI_H
#define LIBUBI_H

#ifdef __cplusplus
extern "C" {
#endif

/* Case A: union with ANONYMOUS struct containing bitfields + raw scalar. */
typedef union UBitView {
    struct {
        unsigned int lo : 8;
        unsigned int hi : 8;
    } bits;
    unsigned short word;
} UBitView;

/* Case B: union whose member is a NAMED struct containing bitfields. */
typedef struct BitFields {
    unsigned int lo : 8;
    unsigned int hi : 8;
} BitFields;

typedef union UBitNamed {
    BitFields bf;
    unsigned short word;
} UBitNamed;

/* Case C: container struct holding a bitfield union. */
typedef struct UBitHolder {
    int tag;
    UBitView uv;
} UBitHolder;

UBitView*  ubi_alloc_view(void);
void       ubi_free_view(UBitView* p);
void       ubi_set_word(UBitView* p, unsigned short v);
unsigned short ubi_get_word(UBitView* p);
void       ubi_set_lo(UBitView* p, unsigned int v);
unsigned int ubi_get_lo(UBitView* p);
void       ubi_set_hi(UBitView* p, unsigned int v);
unsigned int ubi_get_hi(UBitView* p);

UBitNamed* ubi_alloc_named(void);
void       ubi_free_named(UBitNamed* p);
void       ubi_set_named_word(UBitNamed* p, unsigned short v);
unsigned short ubi_get_named_word(UBitNamed* p);

UBitHolder* ubi_alloc_holder(void);
void       ubi_free_holder(UBitHolder* p);
int        ubi_get_holder_tag(UBitHolder* p);
void       ubi_set_holder_tag(UBitHolder* p, int v);

#endif
