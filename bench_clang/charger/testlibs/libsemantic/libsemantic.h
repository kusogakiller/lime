#ifndef LIBSEMANTIC_H
#define LIBSEMANTIC_H

// Phase 1 Iteration 7: Semantic Supplement Layer — generic ABI test fixture.
// Library-agnostic names (sem_*) exercising ownership / nullability / lifetime
// / allocator-deallocator pairing. NO library-specific special casing. The
// SEMANTICS are supplied out-of-band in charger_semantic.toml; the C source
// itself carries NO ownership hints except standard __attribute__((nonnull))
// (which Charger auto-extracts as a fact).

#include <stddef.h>

// Opaque handle type.
typedef struct SemObj { int id; int refcount; } SemObj;
typedef struct SemHandle { int tag; void *priv; } SemHandle;

// --- Test A: owned return + destroy (allocator/deallocator pairing) ---
// sem_create returns an owned SemObj*; sem_destroy consumes it.
SemObj *sem_create(int id);
void sem_destroy(SemObj *obj);

// --- Test B: borrowed return (lifetime depends on a parameter) ---
// sem_get_name returns a pointer into `obj`'s storage; it is borrowed from
// parameter 0. The C signature below is just `const char*` — Charger must NOT
// infer borrowed from the type; it comes from semantic metadata.
const char *sem_get_name(SemObj *obj);

// --- Test C: nullable parameter ---
// Nullability is supplied via semantic metadata (charger_semantic.toml); no
// portable C attribute is required. Charger records it as Unknown-from-AST +
// explicit metadata. (clang's _Nullable postfix qualifier is not accepted by
// this toolchain in C mode, so we rely on the auxiliary metadata here.)
int sem_take_nullable(SemObj *obj);

// --- Test D: nonnull parameter (auto-extracted from __attribute__((nonnull))) ---
int sem_take_nonnull(SemObj *obj) __attribute__((nonnull(1)));

// --- Test E: consumed parameter ---
// sem_consume takes ownership of the passed SemObj* and frees it.
void sem_consume(SemObj *obj);

// --- Tests F & G: callback lifetime (retained vs call-only) ---
// The callback is carried in a struct-with-fn-ptr-field (the supported
// Iteration-2 callback-table mechanism), so Lime can register a real callback.
// `sem_cb_register` STORES the table (retained); `sem_cb_once` uses it only
// during the call (call-only). Charger records the lifetime metadata; it does
// NOT generate different runtime behavior.
typedef struct SemCb {
    void (*on_event)(int value);
} SemCb;

void sem_cb_register(SemCb *cb);   // retained: stores cb, fires later
void sem_cb_fire(int value);       // invokes the stored cb->on_event(value)
void sem_cb_unregister(void);      // clears the stored cb
void sem_cb_once(SemCb *cb, int value); // call-only: invokes cb->on_event(value) now

// --- Test H: opaque handle create/close (resource pairing) ---
SemHandle *sem_handle_create(int tag);
void sem_handle_close(SemHandle *h);

// --- Test I: global pointer with semantic metadata ---
// A global pointer to a shared SemObj. Semantic metadata marks ownership/
// nullability/mutability out-of-band.
extern SemObj *sem_shared;

// --- Test J: unknown semantics remain unknown ---
// These functions intentionally have NO semantic metadata. Charger must not
// infer anything (no name-based guessing).
SemObj *sem_make(int id);
int sem_use(SemObj *obj);
int sem_pick(int which);

#endif // LIBSEMANTIC_H
