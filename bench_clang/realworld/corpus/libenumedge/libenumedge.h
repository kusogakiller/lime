#ifndef PROBE_H
#define PROBE_H

// A. enum width
enum e_u8  : unsigned char { EU8_A = 1,    EU8_B = 200 };
enum e_i16 : short         { EI16_A = 300, EI16_B = -2 };
enum e_def                 { EDEF_A = 70000, EDEF_B = 1 };

int take_enum_u8(enum e_u8 v);
enum e_i16 ret_enum_i16(int v);

// B. bitfield
struct BFProbe {
    unsigned a : 1;
    unsigned b : 7;
    unsigned c : 24;
    int after;
};

// C. packed
struct __attribute__((packed)) PackedProbe {
    char a;
    int b;
    short c;
};
struct NaturalProbe {
    char a;
    int b;
    short c;
};

#endif
