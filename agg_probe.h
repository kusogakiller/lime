typedef struct { int x; int y; } Point2;
typedef struct { Point2 origin; int count; int vals[4]; char name[16]; } Aggregate;
typedef struct { int id; int len; double data[]; } FlexBuf;
void use(Aggregate*a, FlexBuf*f){}
