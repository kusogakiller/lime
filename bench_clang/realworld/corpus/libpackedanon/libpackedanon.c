#include "libpackedanon.h"
#include <stdlib.h>
#include <string.h>

PackedAnon pka_make(void) {
    PackedAnon p;
    memset(&p, 0, sizeof(PackedAnon));
    return p;
}

void pka_free(PackedAnon* p) {
    (void)p;
}

unsigned char pka_get_head(const PackedAnon* p) { return p->head; }
void pka_set_head(PackedAnon* p, unsigned char v) { p->head = v; }
unsigned char pka_get_x(const PackedAnon* p) { return p->x; }
void pka_set_x(PackedAnon* p, unsigned char v) { p->x = v; }
unsigned short pka_get_y(const PackedAnon* p) { return p->y; }
void pka_set_y(PackedAnon* p, unsigned short v) { p->y = v; }
unsigned char pka_get_tail(const PackedAnon* p) { return p->tail; }
void pka_set_tail(PackedAnon* p, unsigned char v) { p->tail = v; }

PackedAnonBit pka_bit_make(void) {
    PackedAnonBit p;
    memset(&p, 0, sizeof(PackedAnonBit));
    return p;
}

void pka_bit_free(PackedAnonBit* p) { (void)p; }

unsigned char pka_bit_get_lead(const PackedAnonBit* p) { return p->lead; }
void pka_bit_set_lead(PackedAnonBit* p, unsigned char v) { p->lead = v; }
unsigned char pka_bit_get_a(const PackedAnonBit* p) { return p->a; }
void pka_bit_set_a(PackedAnonBit* p, unsigned char v) { p->a = v; }
unsigned short pka_bit_get_b(const PackedAnonBit* p) { return p->b; }
void pka_bit_set_b(PackedAnonBit* p, unsigned short v) { p->b = v; }
unsigned char pka_bit_get_trail(const PackedAnonBit* p) { return p->trail; }
void pka_bit_set_trail(PackedAnonBit* p, unsigned char v) { p->trail = v; }

int pka_link_anchor(void) { return 0xDEADBEEF; }