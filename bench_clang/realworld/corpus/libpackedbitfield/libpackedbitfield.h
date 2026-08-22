#ifndef LIBPACKEDBITFIELD_H
#define LIBPACKEDBITFIELD_H

#ifdef __cplusplus
extern "C" {
#endif

/* packed struct + bitfield + trailing ordinary field */
#pragma pack(push, 1)
typedef struct PackedBF {
    unsigned int   a : 3;
    unsigned int   b : 5;
    unsigned int   c : 7;
    unsigned char  tail;
} PackedBF;
#pragma pack(pop)

/* packed + signed bitfield + ordinary field after + bitfield after */
#pragma pack(push, 1)
typedef struct PackedBFMix {
    unsigned char  lead;
    int            x : 6;   /* signed */
    unsigned int   y : 10;
    unsigned short z;       /* ordinary field after bitfields */
    unsigned int   w : 4;   /* bitfield after ordinary field */
} PackedBFMix;
#pragma pack(pop)

int           pbf_get_a(const PackedBF* p);
void          pbf_set_a(PackedBF* p, int v);
int           pbf_get_b(const PackedBF* p);
void          pbf_set_b(PackedBF* p, int v);
int           pbf_get_c(const PackedBF* p);
void          pbf_set_c(PackedBF* p, int v);
unsigned char pbf_get_tail(const PackedBF* p);
void          pbf_set_tail(PackedBF* p, unsigned char v);

int            pbfm_get_x(const PackedBFMix* p);
void           pbfm_set_x(PackedBFMix* p, int v);
unsigned int   pbfm_get_y(const PackedBFMix* p);
void           pbfm_set_y(PackedBFMix* p, unsigned int v);
unsigned short pbfm_get_z(const PackedBFMix* p);
void           pbfm_set_z(PackedBFMix* p, unsigned short v);
unsigned int   pbfm_get_w(const PackedBFMix* p);
void           pbfm_set_w(PackedBFMix* p, unsigned int v);
unsigned char  pbfm_get_lead(const PackedBFMix* p);
void           pbfm_set_lead(PackedBFMix* p, unsigned char v);

#ifdef __cplusplus
}
#endif

#endif
