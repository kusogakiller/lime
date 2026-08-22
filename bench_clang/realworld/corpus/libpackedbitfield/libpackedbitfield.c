#include "libpackedbitfield.h"

int pbf_get_a(const PackedBF* p){ return (int)(p->a); }
void pbf_set_a(PackedBF* p, int v){ p->a = (unsigned int)v; }
int pbf_get_b(const PackedBF* p){ return (int)(p->b); }
void pbf_set_b(PackedBF* p, int v){ p->b = (unsigned int)v; }
int pbf_get_c(const PackedBF* p){ return (int)(p->c); }
void pbf_set_c(PackedBF* p, int v){ p->c = (unsigned int)v; }
unsigned char pbf_get_tail(const PackedBF* p){ return p->tail; }
void pbf_set_tail(PackedBF* p, unsigned char v){ p->tail = v; }

int pbfm_get_x(const PackedBFMix* p){ return p->x; }
void pbfm_set_x(PackedBFMix* p, int v){ p->x = v; }
unsigned int pbfm_get_y(const PackedBFMix* p){ return p->y; }
void pbfm_set_y(PackedBFMix* p, unsigned int v){ p->y = v; }
unsigned short pbfm_get_z(const PackedBFMix* p){ return p->z; }
void pbfm_set_z(PackedBFMix* p, unsigned short v){ p->z = v; }
unsigned int pbfm_get_w(const PackedBFMix* p){ return p->w; }
void pbfm_set_w(PackedBFMix* p, unsigned int v){ p->w = v; }
unsigned char pbfm_get_lead(const PackedBFMix* p){ return p->lead; }
void pbfm_set_lead(PackedBFMix* p, unsigned char v){ p->lead = v; }
