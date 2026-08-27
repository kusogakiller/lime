#ifndef LIBPACKEDANON_H
#define LIBPACKEDANON_H

#ifdef __cplusplus
extern "C" {
#endif

#pragma pack(push, 1)
typedef struct PackedAnon {
    unsigned char head;
    
    struct {
        unsigned char x;
        unsigned short y;
    };
    
    unsigned char tail;
} PackedAnon;
#pragma pack(pop)

#pragma pack(push, 1)
typedef struct PackedAnonBit {
    unsigned char lead;
    
    struct {
        unsigned char a : 3;
        unsigned short b : 12;
    };
    
    unsigned char trail;
} PackedAnonBit;
#pragma pack(pop)

PackedAnon   pka_make(void);
void         pka_free(PackedAnon*);
unsigned char pka_get_head(const PackedAnon* p);
void         pka_set_head(PackedAnon* p, unsigned char v);
unsigned char pka_get_x(const PackedAnon* p);
void         pka_set_x(PackedAnon* p, unsigned char v);
unsigned short pka_get_y(const PackedAnon* p);
void         pka_set_y(PackedAnon* p, unsigned short v);
unsigned char pka_get_tail(const PackedAnon* p);
void         pka_set_tail(PackedAnon* p, unsigned char v);

PackedAnonBit pka_bit_make(void);
void          pka_bit_free(PackedAnonBit*);
unsigned char pka_bit_get_lead(const PackedAnonBit* p);
void          pka_bit_set_lead(PackedAnonBit* p, unsigned char v);
unsigned char pka_bit_get_a(const PackedAnonBit* p);
void          pka_bit_set_a(PackedAnonBit* p, unsigned char v);
unsigned short pka_bit_get_b(const PackedAnonBit* p);
void          pka_bit_set_b(PackedAnonBit* p, unsigned short v);
unsigned char pka_bit_get_trail(const PackedAnonBit* p);
void          pka_bit_set_trail(PackedAnonBit* p, unsigned char v);

int pka_link_anchor(void);

#ifdef __cplusplus
}
#endif

#endif