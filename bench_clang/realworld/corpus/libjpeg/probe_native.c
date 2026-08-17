/* Native cross-check probe for libjpeg-turbo. Links the SAME prebuilt .lib the
 * Charger slice links, and prints the exact ABI literals a real caller must
 * pass to jpeg_CreateCompress: JPEG_LIB_VERSION and the true sizeof the public
 * compress/decompress structs. Output is the source of truth for the Lime slice
 * (no hand-guessed Windows/x64 sizes). */
#include <stdio.h>
#include "jpeglib.h"

int main(void) {
    printf("JPEG_LIB_VERSION=%d\n", JPEG_LIB_VERSION);
    printf("sizeof_jpeg_compress_struct=%zu\n", sizeof(struct jpeg_compress_struct));
    printf("sizeof_jpeg_decompress_struct=%zu\n", sizeof(struct jpeg_decompress_struct));
    printf("sizeof_jpeg_error_mgr=%zu\n", sizeof(struct jpeg_error_mgr));
    /* Perform the canonical create/destroy sequence to prove the library is
     * callable through the real ABI. jpeg_std_error fills the error manager;
     * jpeg_CreateCompress validates (version, structsize); jpeg_destroy_compress
     * tears it down. If any step mis-validates, the JPEG library longjmps. */
    struct jpeg_compress_struct cinfo;
    struct jpeg_error_mgr jerr;
    cinfo.err = jpeg_std_error(&jerr);
    jpeg_CreateCompress(&cinfo, JPEG_LIB_VERSION, sizeof(struct jpeg_compress_struct));
    jpeg_destroy_compress(&cinfo);
    printf("CREATE_DESTROY_OK\n");
    return 0;
}
