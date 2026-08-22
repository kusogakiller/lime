# Iteration 17 — Phase 2B finding (ffmpeg_media_objects)

## Goal
Prove AVPacket / AVFrame / AVCodecParameters opaque lifecycle (alloc/free/unref)
via Charger generic layer, corpus-only (no charger.rs change).

## Result: PARTIAL GREEN
- charger install SUCCEEDED (functions: 697, structs: 48).
- AVPacket lifecycle:  GREEN  (av_packet_alloc/free/unref surfaced as Opaque(AVPacket))
- AVFrame  lifecycle:  GREEN  (av_frame_alloc/free/unref surfaced as Opaque(AVFrame))
- AVCodecParameters lifecycle:  BLOCKED at CType normalization (see below)

## Interface evidence (Source of Truth = installed store manifest + lime-iface.lime)
- av_packet_alloc()     -> Opaque(AVPacket)            [present]
- av_packet_free(Opaque(AVPacket)) -> Unit "lime_take_av_packet_free"  [present]
- av_packet_unref(Opaque(AVPacket)) -> Unit            [present]
- av_frame_alloc()      -> Opaque(AVFrame)             [present]
- av_frame_free(Opaque(AVFrame)) -> Unit "lime_take_av_frame_free"      [present]
- av_frame_unref(Opaque(AVFrame)) -> Unit              [present]
- avcodec_parameters_alloc() -> ???                     [ABSENT from manifest+iface]
- avcodec_parameters_free(???) -> ???                    [ABSENT from manifest+iface]
- avcodec_parameters_from_context / to_context: present (use Opaque(AVCodecParameters) in signature)

## Root-cause classification (NOT corpus-config solvable)
FAILURE TYPE: CType normalization gap (charger.rs), not AST extraction, not adapter gen.

Evidence chain:
1. Symbols ARE in the clang AST: `libavcodec/codec_par.c` FunctionDecl
   avcodec_parameters_alloc (line 227) and avcodec_parameters_free (line 233),
   qualType `AVCodecParameters *(void)` / `void (AVCodecParameters **)`.
   They are declared in codec_par.h (included by avcodec.h:53).
2. Charger's function extractor (charger.rs:1870) pushes every non-reserved
   FunctionDecl unconditionally, yet the manifest function list contains
   avcodec_parameters_from_context / to_context (same TU) but NOT alloc/free
   (grep count = 0 in manifest + iface). So the drop is post-extraction.
3. Distinguishing factor: AVCodecParameters is surfaced as a COMPLETE struct
   (`struct AVCodecParameters { ... }`, iface line 623) because codec_par.h
   carries the full field list. AVPacket / AVFrame are surfaced as Opaque
   handles. A function returning `AVCodecParameters *` (= Pointer(Struct)) is
   handled by the struct-by-value / struct-return machinery (charger.rs 2762-2845)
   and never emitted as an opaque-handle lifecycle function, so alloc/free drop
   out of the surfaced API.

This is the same generic boundary as the known "complete struct layout" limit:
pointer-to-complete-struct is not normalized to Opaque the way pointer-to-incomplete
/ opaque handle is. AVPacket/AVFrame happened to normalize as Opaque; AVCodecParameters
normalized as complete struct.

## Why corpus-only cannot fix it
Making AVCodecParameters incomplete in the corpus header copy would let charger treat
its pointer as Opaque (surfacing alloc/free), BUT codec_par.c (the compiled TU) needs
the full definition to compile, and it includes the same corpus header -> native build
of codec_par.c would break. No corpus-only lever separates "charger's parse view" from
"native compile view" for a single header. Hence charger.rs normalization change required.

## Proposed charger.rs fix (design, NOT applied this step)
Normalize `Pointer(Struct(complete))` used as an opaque handle in function signatures
(the same way Pointer(Opaque) already is): for any parameter/return that is a bare
pointer to a named record, surface it as Opaque(Name) at the function-signature level,
independent of whether the record is also surfaced as a complete struct with field
accessors. This keeps struct field access (make/get/set shims) AND adds the missing
pointer-based lifecycle functions (alloc/free). Generic — derived from pointer-to-record
type, no library names.

## Corpus state (for Phase 2C/D resume)
- Real TUs: libavcodec/packet.c, libavcodec/codec_par.c, libavutil/frame.c,
  libavutil/log2_tab.c (ff_log2_tab internal symbol required by adapter gen).
- Headers: full libavutil/*.h + libavutil/avconfig.h, full libavcodec/*.h, config.h
  (stub; HAS HAVE_AV_CONFIG_H 1, ARCH_X86 0, -include config.h in build_flags).
- Corpus-only edit: intmath.h `extern const uint8_t ff_log2_tab[256];` removed
  (adapter gen mis-identified the internal global as a struct-field accessor; removing
  the extern decl stops that, ff_log2_tab still provided by log2_tab.c).
- 8/8 lifecycle APIs present in clang AST; 6/8 surfaced (AVCodecParameters blocked by
  charger.rs normalization).
