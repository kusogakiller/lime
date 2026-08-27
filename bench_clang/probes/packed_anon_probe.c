/*
 * Native C probe: packed outer + nested anonymous struct
 * Tests: sizeof, alignof, offsetof, field access, bitfield if useful
 */

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <stdalign.h>

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

int main() {
    printf("=== PackedAnon ===\n");
    printf("sizeof = %zu\n", sizeof(PackedAnon));
    printf("alignof = %zu\n", alignof(PackedAnon));
    printf("offsetof(head) = %zu\n", offsetof(PackedAnon, head));
    printf("offsetof(x) = %zu\n", offsetof(PackedAnon, x));
    printf("offsetof(y) = %zu\n", offsetof(PackedAnon, y));
    printf("offsetof(tail) = %zu\n", offsetof(PackedAnon, tail));
    
    PackedAnon p = {0};
    p.head = 0x11;
    p.x = 0x22;
    p.y = 0x3344;
    p.tail = 0x55;
    printf("head=0x%02X x=0x%02X y=0x%04X tail=0x%02X\n", p.head, p.x, p.y, p.tail);
    
    printf("\n=== PackedAnonBit ===\n");
    printf("sizeof = %zu\n", sizeof(PackedAnonBit));
    printf("alignof = %zu\n", alignof(PackedAnonBit));
    printf("offsetof(lead) = %zu\n", offsetof(PackedAnonBit, lead));
    // bitfields don't have offsetof in C
    printf("offsetof(trail) = %zu\n", offsetof(PackedAnonBit, trail));
    
    PackedAnonBit q = {0};
    q.lead = 0xAA;
    q.a = 0x5;
    q.b = 0xABC;
    q.trail = 0xBB;
    printf("lead=0x%02X a=%u b=%u trail=0x%02X\n", q.lead, q.a, q.b, q.trail);
    
    printf("\nNATIVE_PROBE_OK\n");
    return 0;
}