/* Charger-generated adapter shims (out-param + null-callback + const + union/bitfield accessors + variadic). DO NOT EDIT. */
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include "jpeglib.h"
void* lime_make_JQUANT_TBL(void) { return (void*)calloc(1, sizeof(JQUANT_TBL)); }
unsigned short lime_get_JQUANT_TBL_quantval_i(JQUANT_TBL* u, int i) { return (unsigned short)u->quantval[i]; }
void lime_set_JQUANT_TBL_quantval_i(JQUANT_TBL* u, int i, unsigned short v) { u->quantval[i] = (unsigned short)v; }
unsigned char lime_get_JQUANT_TBL_sent_table(JQUANT_TBL* u) { return (unsigned char)u->sent_table; }
void lime_set_JQUANT_TBL_sent_table(JQUANT_TBL* u, unsigned char v) { u->sent_table = (unsigned char)v; }

void* lime_make_JHUFF_TBL(void) { return (void*)calloc(1, sizeof(JHUFF_TBL)); }
unsigned char lime_get_JHUFF_TBL_bits_i(JHUFF_TBL* u, int i) { return (unsigned char)u->bits[i]; }
void lime_set_JHUFF_TBL_bits_i(JHUFF_TBL* u, int i, unsigned char v) { u->bits[i] = (unsigned char)v; }
unsigned char lime_get_JHUFF_TBL_huffval_i(JHUFF_TBL* u, int i) { return (unsigned char)u->huffval[i]; }
void lime_set_JHUFF_TBL_huffval_i(JHUFF_TBL* u, int i, unsigned char v) { u->huffval[i] = (unsigned char)v; }
unsigned char lime_get_JHUFF_TBL_sent_table(JHUFF_TBL* u) { return (unsigned char)u->sent_table; }
void lime_set_JHUFF_TBL_sent_table(JHUFF_TBL* u, unsigned char v) { u->sent_table = (unsigned char)v; }

void* lime_make_jpeg_component_info(void) { return (void*)calloc(1, sizeof(jpeg_component_info)); }
int lime_get_jpeg_component_info_component_id(jpeg_component_info* u) { return (int)u->component_id; }
void lime_set_jpeg_component_info_component_id(jpeg_component_info* u, int v) { u->component_id = (int)v; }
int lime_get_jpeg_component_info_component_index(jpeg_component_info* u) { return (int)u->component_index; }
void lime_set_jpeg_component_info_component_index(jpeg_component_info* u, int v) { u->component_index = (int)v; }
int lime_get_jpeg_component_info_h_samp_factor(jpeg_component_info* u) { return (int)u->h_samp_factor; }
void lime_set_jpeg_component_info_h_samp_factor(jpeg_component_info* u, int v) { u->h_samp_factor = (int)v; }
int lime_get_jpeg_component_info_v_samp_factor(jpeg_component_info* u) { return (int)u->v_samp_factor; }
void lime_set_jpeg_component_info_v_samp_factor(jpeg_component_info* u, int v) { u->v_samp_factor = (int)v; }
int lime_get_jpeg_component_info_quant_tbl_no(jpeg_component_info* u) { return (int)u->quant_tbl_no; }
void lime_set_jpeg_component_info_quant_tbl_no(jpeg_component_info* u, int v) { u->quant_tbl_no = (int)v; }
int lime_get_jpeg_component_info_dc_tbl_no(jpeg_component_info* u) { return (int)u->dc_tbl_no; }
void lime_set_jpeg_component_info_dc_tbl_no(jpeg_component_info* u, int v) { u->dc_tbl_no = (int)v; }
int lime_get_jpeg_component_info_ac_tbl_no(jpeg_component_info* u) { return (int)u->ac_tbl_no; }
void lime_set_jpeg_component_info_ac_tbl_no(jpeg_component_info* u, int v) { u->ac_tbl_no = (int)v; }
int lime_get_jpeg_component_info_width_in_blocks(jpeg_component_info* u) { return (int)u->width_in_blocks; }
void lime_set_jpeg_component_info_width_in_blocks(jpeg_component_info* u, int v) { u->width_in_blocks = (int)v; }
int lime_get_jpeg_component_info_height_in_blocks(jpeg_component_info* u) { return (int)u->height_in_blocks; }
void lime_set_jpeg_component_info_height_in_blocks(jpeg_component_info* u, int v) { u->height_in_blocks = (int)v; }
int lime_get_jpeg_component_info_DCT_h_scaled_size(jpeg_component_info* u) { return (int)u->DCT_h_scaled_size; }
void lime_set_jpeg_component_info_DCT_h_scaled_size(jpeg_component_info* u, int v) { u->DCT_h_scaled_size = (int)v; }
int lime_get_jpeg_component_info_DCT_v_scaled_size(jpeg_component_info* u) { return (int)u->DCT_v_scaled_size; }
void lime_set_jpeg_component_info_DCT_v_scaled_size(jpeg_component_info* u, int v) { u->DCT_v_scaled_size = (int)v; }
int lime_get_jpeg_component_info_downsampled_width(jpeg_component_info* u) { return (int)u->downsampled_width; }
void lime_set_jpeg_component_info_downsampled_width(jpeg_component_info* u, int v) { u->downsampled_width = (int)v; }
int lime_get_jpeg_component_info_downsampled_height(jpeg_component_info* u) { return (int)u->downsampled_height; }
void lime_set_jpeg_component_info_downsampled_height(jpeg_component_info* u, int v) { u->downsampled_height = (int)v; }
unsigned char lime_get_jpeg_component_info_component_needed(jpeg_component_info* u) { return (unsigned char)u->component_needed; }
void lime_set_jpeg_component_info_component_needed(jpeg_component_info* u, unsigned char v) { u->component_needed = (unsigned char)v; }
int lime_get_jpeg_component_info_MCU_width(jpeg_component_info* u) { return (int)u->MCU_width; }
void lime_set_jpeg_component_info_MCU_width(jpeg_component_info* u, int v) { u->MCU_width = (int)v; }
int lime_get_jpeg_component_info_MCU_height(jpeg_component_info* u) { return (int)u->MCU_height; }
void lime_set_jpeg_component_info_MCU_height(jpeg_component_info* u, int v) { u->MCU_height = (int)v; }
int lime_get_jpeg_component_info_MCU_blocks(jpeg_component_info* u) { return (int)u->MCU_blocks; }
void lime_set_jpeg_component_info_MCU_blocks(jpeg_component_info* u, int v) { u->MCU_blocks = (int)v; }
int lime_get_jpeg_component_info_MCU_sample_width(jpeg_component_info* u) { return (int)u->MCU_sample_width; }
void lime_set_jpeg_component_info_MCU_sample_width(jpeg_component_info* u, int v) { u->MCU_sample_width = (int)v; }
int lime_get_jpeg_component_info_last_col_width(jpeg_component_info* u) { return (int)u->last_col_width; }
void lime_set_jpeg_component_info_last_col_width(jpeg_component_info* u, int v) { u->last_col_width = (int)v; }
int lime_get_jpeg_component_info_last_row_height(jpeg_component_info* u) { return (int)u->last_row_height; }
void lime_set_jpeg_component_info_last_row_height(jpeg_component_info* u, int v) { u->last_row_height = (int)v; }
void* lime_get_jpeg_component_info_quant_table(jpeg_component_info* u) { return (void*)u->quant_table; }
void lime_set_jpeg_component_info_quant_table(jpeg_component_info* u, void* v) { u->quant_table = (void*)v; }
void* lime_get_jpeg_component_info_dct_table(jpeg_component_info* u) { return (void*)u->dct_table; }
void lime_set_jpeg_component_info_dct_table(jpeg_component_info* u, void* v) { u->dct_table = (void*)v; }

void* lime_make_jpeg_scan_info(void) { return (void*)calloc(1, sizeof(jpeg_scan_info)); }
int lime_get_jpeg_scan_info_comps_in_scan(jpeg_scan_info* u) { return (int)u->comps_in_scan; }
void lime_set_jpeg_scan_info_comps_in_scan(jpeg_scan_info* u, int v) { u->comps_in_scan = (int)v; }
int lime_get_jpeg_scan_info_component_index_i(jpeg_scan_info* u, int i) { return (int)u->component_index[i]; }
void lime_set_jpeg_scan_info_component_index_i(jpeg_scan_info* u, int i, int v) { u->component_index[i] = (int)v; }
int lime_get_jpeg_scan_info_Ss(jpeg_scan_info* u) { return (int)u->Ss; }
void lime_set_jpeg_scan_info_Ss(jpeg_scan_info* u, int v) { u->Ss = (int)v; }
int lime_get_jpeg_scan_info_Se(jpeg_scan_info* u) { return (int)u->Se; }
void lime_set_jpeg_scan_info_Se(jpeg_scan_info* u, int v) { u->Se = (int)v; }
int lime_get_jpeg_scan_info_Ah(jpeg_scan_info* u) { return (int)u->Ah; }
void lime_set_jpeg_scan_info_Ah(jpeg_scan_info* u, int v) { u->Ah = (int)v; }
int lime_get_jpeg_scan_info_Al(jpeg_scan_info* u) { return (int)u->Al; }
void lime_set_jpeg_scan_info_Al(jpeg_scan_info* u, int v) { u->Al = (int)v; }

void* lime_make_jpeg_marker_struct(void) { return (void*)calloc(1, sizeof(struct jpeg_marker_struct)); }
void* lime_get_jpeg_marker_struct_next(struct jpeg_marker_struct* u) { return (void*)&u->next; }
void lime_set_jpeg_marker_struct_next(struct jpeg_marker_struct* u, void* v) { memcpy(&u->next, v, sizeof(u->next)); }
unsigned char lime_get_jpeg_marker_struct_marker(struct jpeg_marker_struct* u) { return (unsigned char)u->marker; }
void lime_set_jpeg_marker_struct_marker(struct jpeg_marker_struct* u, unsigned char v) { u->marker = (unsigned char)v; }
int lime_get_jpeg_marker_struct_original_length(struct jpeg_marker_struct* u) { return (int)u->original_length; }
void lime_set_jpeg_marker_struct_original_length(struct jpeg_marker_struct* u, int v) { u->original_length = (int)v; }
int lime_get_jpeg_marker_struct_data_length(struct jpeg_marker_struct* u) { return (int)u->data_length; }
void lime_set_jpeg_marker_struct_data_length(struct jpeg_marker_struct* u, int v) { u->data_length = (int)v; }
void* lime_get_jpeg_marker_struct_data(struct jpeg_marker_struct* u) { return (void*)u->data; }
void lime_set_jpeg_marker_struct_data(struct jpeg_marker_struct* u, void* v) { u->data = (void*)v; }

void* lime_make_jpeg_common_struct(void) { return (void*)calloc(1, sizeof(struct jpeg_common_struct)); }
void* lime_get_jpeg_common_struct_err(struct jpeg_common_struct* u) { return (void*)u->err; }
void lime_set_jpeg_common_struct_err(struct jpeg_common_struct* u, void* v) { u->err = (void*)v; }
void* lime_get_jpeg_common_struct_mem(struct jpeg_common_struct* u) { return (void*)u->mem; }
void lime_set_jpeg_common_struct_mem(struct jpeg_common_struct* u, void* v) { u->mem = (void*)v; }
void* lime_get_jpeg_common_struct_progress(struct jpeg_common_struct* u) { return (void*)u->progress; }
void lime_set_jpeg_common_struct_progress(struct jpeg_common_struct* u, void* v) { u->progress = (void*)v; }
void* lime_get_jpeg_common_struct_client_data(struct jpeg_common_struct* u) { return (void*)u->client_data; }
void lime_set_jpeg_common_struct_client_data(struct jpeg_common_struct* u, void* v) { u->client_data = (void*)v; }
unsigned char lime_get_jpeg_common_struct_is_decompressor(struct jpeg_common_struct* u) { return (unsigned char)u->is_decompressor; }
void lime_set_jpeg_common_struct_is_decompressor(struct jpeg_common_struct* u, unsigned char v) { u->is_decompressor = (unsigned char)v; }
int lime_get_jpeg_common_struct_global_state(struct jpeg_common_struct* u) { return (int)u->global_state; }
void lime_set_jpeg_common_struct_global_state(struct jpeg_common_struct* u, int v) { u->global_state = (int)v; }

void* lime_make_jpeg_compress_struct(void) { return (void*)calloc(1, sizeof(struct jpeg_compress_struct)); }
void* lime_get_jpeg_compress_struct_err(struct jpeg_compress_struct* u) { return (void*)u->err; }
void lime_set_jpeg_compress_struct_err(struct jpeg_compress_struct* u, void* v) { u->err = (void*)v; }
void* lime_get_jpeg_compress_struct_mem(struct jpeg_compress_struct* u) { return (void*)u->mem; }
void lime_set_jpeg_compress_struct_mem(struct jpeg_compress_struct* u, void* v) { u->mem = (void*)v; }
void* lime_get_jpeg_compress_struct_progress(struct jpeg_compress_struct* u) { return (void*)u->progress; }
void lime_set_jpeg_compress_struct_progress(struct jpeg_compress_struct* u, void* v) { u->progress = (void*)v; }
void* lime_get_jpeg_compress_struct_client_data(struct jpeg_compress_struct* u) { return (void*)u->client_data; }
void lime_set_jpeg_compress_struct_client_data(struct jpeg_compress_struct* u, void* v) { u->client_data = (void*)v; }
unsigned char lime_get_jpeg_compress_struct_is_decompressor(struct jpeg_compress_struct* u) { return (unsigned char)u->is_decompressor; }
void lime_set_jpeg_compress_struct_is_decompressor(struct jpeg_compress_struct* u, unsigned char v) { u->is_decompressor = (unsigned char)v; }
int lime_get_jpeg_compress_struct_global_state(struct jpeg_compress_struct* u) { return (int)u->global_state; }
void lime_set_jpeg_compress_struct_global_state(struct jpeg_compress_struct* u, int v) { u->global_state = (int)v; }
void* lime_get_jpeg_compress_struct_dest(struct jpeg_compress_struct* u) { return (void*)u->dest; }
void lime_set_jpeg_compress_struct_dest(struct jpeg_compress_struct* u, void* v) { u->dest = (void*)v; }
int lime_get_jpeg_compress_struct_image_width(struct jpeg_compress_struct* u) { return (int)u->image_width; }
void lime_set_jpeg_compress_struct_image_width(struct jpeg_compress_struct* u, int v) { u->image_width = (int)v; }
int lime_get_jpeg_compress_struct_image_height(struct jpeg_compress_struct* u) { return (int)u->image_height; }
void lime_set_jpeg_compress_struct_image_height(struct jpeg_compress_struct* u, int v) { u->image_height = (int)v; }
int lime_get_jpeg_compress_struct_input_components(struct jpeg_compress_struct* u) { return (int)u->input_components; }
void lime_set_jpeg_compress_struct_input_components(struct jpeg_compress_struct* u, int v) { u->input_components = (int)v; }
int lime_get_jpeg_compress_struct_in_color_space(struct jpeg_compress_struct* u) { return (int)u->in_color_space; }
void lime_set_jpeg_compress_struct_in_color_space(struct jpeg_compress_struct* u, int v) { u->in_color_space = (int)v; }
double lime_get_jpeg_compress_struct_input_gamma(struct jpeg_compress_struct* u) { return (double)u->input_gamma; }
void lime_set_jpeg_compress_struct_input_gamma(struct jpeg_compress_struct* u, double v) { u->input_gamma = (double)v; }
int lime_get_jpeg_compress_struct_scale_num(struct jpeg_compress_struct* u) { return (int)u->scale_num; }
void lime_set_jpeg_compress_struct_scale_num(struct jpeg_compress_struct* u, int v) { u->scale_num = (int)v; }
int lime_get_jpeg_compress_struct_scale_denom(struct jpeg_compress_struct* u) { return (int)u->scale_denom; }
void lime_set_jpeg_compress_struct_scale_denom(struct jpeg_compress_struct* u, int v) { u->scale_denom = (int)v; }
int lime_get_jpeg_compress_struct_jpeg_width(struct jpeg_compress_struct* u) { return (int)u->jpeg_width; }
void lime_set_jpeg_compress_struct_jpeg_width(struct jpeg_compress_struct* u, int v) { u->jpeg_width = (int)v; }
int lime_get_jpeg_compress_struct_jpeg_height(struct jpeg_compress_struct* u) { return (int)u->jpeg_height; }
void lime_set_jpeg_compress_struct_jpeg_height(struct jpeg_compress_struct* u, int v) { u->jpeg_height = (int)v; }
int lime_get_jpeg_compress_struct_data_precision(struct jpeg_compress_struct* u) { return (int)u->data_precision; }
void lime_set_jpeg_compress_struct_data_precision(struct jpeg_compress_struct* u, int v) { u->data_precision = (int)v; }
int lime_get_jpeg_compress_struct_num_components(struct jpeg_compress_struct* u) { return (int)u->num_components; }
void lime_set_jpeg_compress_struct_num_components(struct jpeg_compress_struct* u, int v) { u->num_components = (int)v; }
int lime_get_jpeg_compress_struct_jpeg_color_space(struct jpeg_compress_struct* u) { return (int)u->jpeg_color_space; }
void lime_set_jpeg_compress_struct_jpeg_color_space(struct jpeg_compress_struct* u, int v) { u->jpeg_color_space = (int)v; }
void* lime_get_jpeg_compress_struct_comp_info(struct jpeg_compress_struct* u) { return (void*)u->comp_info; }
void lime_set_jpeg_compress_struct_comp_info(struct jpeg_compress_struct* u, void* v) { u->comp_info = (void*)v; }
struct JQUANT_TBL* lime_get_jpeg_compress_struct_quant_tbl_ptrs_i(struct jpeg_compress_struct* u, int i) { return (struct JQUANT_TBL*)u->quant_tbl_ptrs[i]; }
void lime_set_jpeg_compress_struct_quant_tbl_ptrs_i(struct jpeg_compress_struct* u, int i, struct JQUANT_TBL* v) { u->quant_tbl_ptrs[i] = (struct JQUANT_TBL*)v; }
int lime_get_jpeg_compress_struct_q_scale_factor_i(struct jpeg_compress_struct* u, int i) { return (int)u->q_scale_factor[i]; }
void lime_set_jpeg_compress_struct_q_scale_factor_i(struct jpeg_compress_struct* u, int i, int v) { u->q_scale_factor[i] = (int)v; }
struct JHUFF_TBL* lime_get_jpeg_compress_struct_dc_huff_tbl_ptrs_i(struct jpeg_compress_struct* u, int i) { return (struct JHUFF_TBL*)u->dc_huff_tbl_ptrs[i]; }
void lime_set_jpeg_compress_struct_dc_huff_tbl_ptrs_i(struct jpeg_compress_struct* u, int i, struct JHUFF_TBL* v) { u->dc_huff_tbl_ptrs[i] = (struct JHUFF_TBL*)v; }
struct JHUFF_TBL* lime_get_jpeg_compress_struct_ac_huff_tbl_ptrs_i(struct jpeg_compress_struct* u, int i) { return (struct JHUFF_TBL*)u->ac_huff_tbl_ptrs[i]; }
void lime_set_jpeg_compress_struct_ac_huff_tbl_ptrs_i(struct jpeg_compress_struct* u, int i, struct JHUFF_TBL* v) { u->ac_huff_tbl_ptrs[i] = (struct JHUFF_TBL*)v; }
unsigned char lime_get_jpeg_compress_struct_arith_dc_L_i(struct jpeg_compress_struct* u, int i) { return (unsigned char)u->arith_dc_L[i]; }
void lime_set_jpeg_compress_struct_arith_dc_L_i(struct jpeg_compress_struct* u, int i, unsigned char v) { u->arith_dc_L[i] = (unsigned char)v; }
unsigned char lime_get_jpeg_compress_struct_arith_dc_U_i(struct jpeg_compress_struct* u, int i) { return (unsigned char)u->arith_dc_U[i]; }
void lime_set_jpeg_compress_struct_arith_dc_U_i(struct jpeg_compress_struct* u, int i, unsigned char v) { u->arith_dc_U[i] = (unsigned char)v; }
unsigned char lime_get_jpeg_compress_struct_arith_ac_K_i(struct jpeg_compress_struct* u, int i) { return (unsigned char)u->arith_ac_K[i]; }
void lime_set_jpeg_compress_struct_arith_ac_K_i(struct jpeg_compress_struct* u, int i, unsigned char v) { u->arith_ac_K[i] = (unsigned char)v; }
int lime_get_jpeg_compress_struct_num_scans(struct jpeg_compress_struct* u) { return (int)u->num_scans; }
void lime_set_jpeg_compress_struct_num_scans(struct jpeg_compress_struct* u, int v) { u->num_scans = (int)v; }
void* lime_get_jpeg_compress_struct_scan_info(struct jpeg_compress_struct* u) { return (void*)u->scan_info; }
void lime_set_jpeg_compress_struct_scan_info(struct jpeg_compress_struct* u, void* v) { u->scan_info = (void*)v; }
unsigned char lime_get_jpeg_compress_struct_raw_data_in(struct jpeg_compress_struct* u) { return (unsigned char)u->raw_data_in; }
void lime_set_jpeg_compress_struct_raw_data_in(struct jpeg_compress_struct* u, unsigned char v) { u->raw_data_in = (unsigned char)v; }
unsigned char lime_get_jpeg_compress_struct_arith_code(struct jpeg_compress_struct* u) { return (unsigned char)u->arith_code; }
void lime_set_jpeg_compress_struct_arith_code(struct jpeg_compress_struct* u, unsigned char v) { u->arith_code = (unsigned char)v; }
unsigned char lime_get_jpeg_compress_struct_optimize_coding(struct jpeg_compress_struct* u) { return (unsigned char)u->optimize_coding; }
void lime_set_jpeg_compress_struct_optimize_coding(struct jpeg_compress_struct* u, unsigned char v) { u->optimize_coding = (unsigned char)v; }
unsigned char lime_get_jpeg_compress_struct_CCIR601_sampling(struct jpeg_compress_struct* u) { return (unsigned char)u->CCIR601_sampling; }
void lime_set_jpeg_compress_struct_CCIR601_sampling(struct jpeg_compress_struct* u, unsigned char v) { u->CCIR601_sampling = (unsigned char)v; }
unsigned char lime_get_jpeg_compress_struct_do_fancy_downsampling(struct jpeg_compress_struct* u) { return (unsigned char)u->do_fancy_downsampling; }
void lime_set_jpeg_compress_struct_do_fancy_downsampling(struct jpeg_compress_struct* u, unsigned char v) { u->do_fancy_downsampling = (unsigned char)v; }
int lime_get_jpeg_compress_struct_smoothing_factor(struct jpeg_compress_struct* u) { return (int)u->smoothing_factor; }
void lime_set_jpeg_compress_struct_smoothing_factor(struct jpeg_compress_struct* u, int v) { u->smoothing_factor = (int)v; }
int lime_get_jpeg_compress_struct_dct_method(struct jpeg_compress_struct* u) { return (int)u->dct_method; }
void lime_set_jpeg_compress_struct_dct_method(struct jpeg_compress_struct* u, int v) { u->dct_method = (int)v; }
int lime_get_jpeg_compress_struct_restart_interval(struct jpeg_compress_struct* u) { return (int)u->restart_interval; }
void lime_set_jpeg_compress_struct_restart_interval(struct jpeg_compress_struct* u, int v) { u->restart_interval = (int)v; }
int lime_get_jpeg_compress_struct_restart_in_rows(struct jpeg_compress_struct* u) { return (int)u->restart_in_rows; }
void lime_set_jpeg_compress_struct_restart_in_rows(struct jpeg_compress_struct* u, int v) { u->restart_in_rows = (int)v; }
unsigned char lime_get_jpeg_compress_struct_write_JFIF_header(struct jpeg_compress_struct* u) { return (unsigned char)u->write_JFIF_header; }
void lime_set_jpeg_compress_struct_write_JFIF_header(struct jpeg_compress_struct* u, unsigned char v) { u->write_JFIF_header = (unsigned char)v; }
unsigned char lime_get_jpeg_compress_struct_JFIF_major_version(struct jpeg_compress_struct* u) { return (unsigned char)u->JFIF_major_version; }
void lime_set_jpeg_compress_struct_JFIF_major_version(struct jpeg_compress_struct* u, unsigned char v) { u->JFIF_major_version = (unsigned char)v; }
unsigned char lime_get_jpeg_compress_struct_JFIF_minor_version(struct jpeg_compress_struct* u) { return (unsigned char)u->JFIF_minor_version; }
void lime_set_jpeg_compress_struct_JFIF_minor_version(struct jpeg_compress_struct* u, unsigned char v) { u->JFIF_minor_version = (unsigned char)v; }
unsigned char lime_get_jpeg_compress_struct_density_unit(struct jpeg_compress_struct* u) { return (unsigned char)u->density_unit; }
void lime_set_jpeg_compress_struct_density_unit(struct jpeg_compress_struct* u, unsigned char v) { u->density_unit = (unsigned char)v; }
unsigned short lime_get_jpeg_compress_struct_X_density(struct jpeg_compress_struct* u) { return (unsigned short)u->X_density; }
void lime_set_jpeg_compress_struct_X_density(struct jpeg_compress_struct* u, unsigned short v) { u->X_density = (unsigned short)v; }
unsigned short lime_get_jpeg_compress_struct_Y_density(struct jpeg_compress_struct* u) { return (unsigned short)u->Y_density; }
void lime_set_jpeg_compress_struct_Y_density(struct jpeg_compress_struct* u, unsigned short v) { u->Y_density = (unsigned short)v; }
unsigned char lime_get_jpeg_compress_struct_write_Adobe_marker(struct jpeg_compress_struct* u) { return (unsigned char)u->write_Adobe_marker; }
void lime_set_jpeg_compress_struct_write_Adobe_marker(struct jpeg_compress_struct* u, unsigned char v) { u->write_Adobe_marker = (unsigned char)v; }
int lime_get_jpeg_compress_struct_next_scanline(struct jpeg_compress_struct* u) { return (int)u->next_scanline; }
void lime_set_jpeg_compress_struct_next_scanline(struct jpeg_compress_struct* u, int v) { u->next_scanline = (int)v; }
unsigned char lime_get_jpeg_compress_struct_progressive_mode(struct jpeg_compress_struct* u) { return (unsigned char)u->progressive_mode; }
void lime_set_jpeg_compress_struct_progressive_mode(struct jpeg_compress_struct* u, unsigned char v) { u->progressive_mode = (unsigned char)v; }
int lime_get_jpeg_compress_struct_max_h_samp_factor(struct jpeg_compress_struct* u) { return (int)u->max_h_samp_factor; }
void lime_set_jpeg_compress_struct_max_h_samp_factor(struct jpeg_compress_struct* u, int v) { u->max_h_samp_factor = (int)v; }
int lime_get_jpeg_compress_struct_max_v_samp_factor(struct jpeg_compress_struct* u) { return (int)u->max_v_samp_factor; }
void lime_set_jpeg_compress_struct_max_v_samp_factor(struct jpeg_compress_struct* u, int v) { u->max_v_samp_factor = (int)v; }
int lime_get_jpeg_compress_struct_min_DCT_h_scaled_size(struct jpeg_compress_struct* u) { return (int)u->min_DCT_h_scaled_size; }
void lime_set_jpeg_compress_struct_min_DCT_h_scaled_size(struct jpeg_compress_struct* u, int v) { u->min_DCT_h_scaled_size = (int)v; }
int lime_get_jpeg_compress_struct_min_DCT_v_scaled_size(struct jpeg_compress_struct* u) { return (int)u->min_DCT_v_scaled_size; }
void lime_set_jpeg_compress_struct_min_DCT_v_scaled_size(struct jpeg_compress_struct* u, int v) { u->min_DCT_v_scaled_size = (int)v; }
int lime_get_jpeg_compress_struct_total_iMCU_rows(struct jpeg_compress_struct* u) { return (int)u->total_iMCU_rows; }
void lime_set_jpeg_compress_struct_total_iMCU_rows(struct jpeg_compress_struct* u, int v) { u->total_iMCU_rows = (int)v; }
int lime_get_jpeg_compress_struct_comps_in_scan(struct jpeg_compress_struct* u) { return (int)u->comps_in_scan; }
void lime_set_jpeg_compress_struct_comps_in_scan(struct jpeg_compress_struct* u, int v) { u->comps_in_scan = (int)v; }
struct jpeg_component_info* lime_get_jpeg_compress_struct_cur_comp_info_i(struct jpeg_compress_struct* u, int i) { return (struct jpeg_component_info*)u->cur_comp_info[i]; }
void lime_set_jpeg_compress_struct_cur_comp_info_i(struct jpeg_compress_struct* u, int i, struct jpeg_component_info* v) { u->cur_comp_info[i] = (struct jpeg_component_info*)v; }
int lime_get_jpeg_compress_struct_MCUs_per_row(struct jpeg_compress_struct* u) { return (int)u->MCUs_per_row; }
void lime_set_jpeg_compress_struct_MCUs_per_row(struct jpeg_compress_struct* u, int v) { u->MCUs_per_row = (int)v; }
int lime_get_jpeg_compress_struct_MCU_rows_in_scan(struct jpeg_compress_struct* u) { return (int)u->MCU_rows_in_scan; }
void lime_set_jpeg_compress_struct_MCU_rows_in_scan(struct jpeg_compress_struct* u, int v) { u->MCU_rows_in_scan = (int)v; }
int lime_get_jpeg_compress_struct_blocks_in_MCU(struct jpeg_compress_struct* u) { return (int)u->blocks_in_MCU; }
void lime_set_jpeg_compress_struct_blocks_in_MCU(struct jpeg_compress_struct* u, int v) { u->blocks_in_MCU = (int)v; }
int lime_get_jpeg_compress_struct_MCU_membership_i(struct jpeg_compress_struct* u, int i) { return (int)u->MCU_membership[i]; }
void lime_set_jpeg_compress_struct_MCU_membership_i(struct jpeg_compress_struct* u, int i, int v) { u->MCU_membership[i] = (int)v; }
int lime_get_jpeg_compress_struct_Ss(struct jpeg_compress_struct* u) { return (int)u->Ss; }
void lime_set_jpeg_compress_struct_Ss(struct jpeg_compress_struct* u, int v) { u->Ss = (int)v; }
int lime_get_jpeg_compress_struct_Se(struct jpeg_compress_struct* u) { return (int)u->Se; }
void lime_set_jpeg_compress_struct_Se(struct jpeg_compress_struct* u, int v) { u->Se = (int)v; }
int lime_get_jpeg_compress_struct_Ah(struct jpeg_compress_struct* u) { return (int)u->Ah; }
void lime_set_jpeg_compress_struct_Ah(struct jpeg_compress_struct* u, int v) { u->Ah = (int)v; }
int lime_get_jpeg_compress_struct_Al(struct jpeg_compress_struct* u) { return (int)u->Al; }
void lime_set_jpeg_compress_struct_Al(struct jpeg_compress_struct* u, int v) { u->Al = (int)v; }
int lime_get_jpeg_compress_struct_block_size(struct jpeg_compress_struct* u) { return (int)u->block_size; }
void lime_set_jpeg_compress_struct_block_size(struct jpeg_compress_struct* u, int v) { u->block_size = (int)v; }
void* lime_get_jpeg_compress_struct_natural_order(struct jpeg_compress_struct* u) { return (void*)u->natural_order; }
void lime_set_jpeg_compress_struct_natural_order(struct jpeg_compress_struct* u, void* v) { u->natural_order = (void*)v; }
int lime_get_jpeg_compress_struct_lim_Se(struct jpeg_compress_struct* u) { return (int)u->lim_Se; }
void lime_set_jpeg_compress_struct_lim_Se(struct jpeg_compress_struct* u, int v) { u->lim_Se = (int)v; }
void* lime_get_jpeg_compress_struct_master(struct jpeg_compress_struct* u) { return (void*)u->master; }
void lime_set_jpeg_compress_struct_master(struct jpeg_compress_struct* u, void* v) { u->master = (void*)v; }
void* lime_get_jpeg_compress_struct_main(struct jpeg_compress_struct* u) { return (void*)u->main; }
void lime_set_jpeg_compress_struct_main(struct jpeg_compress_struct* u, void* v) { u->main = (void*)v; }
void* lime_get_jpeg_compress_struct_prep(struct jpeg_compress_struct* u) { return (void*)u->prep; }
void lime_set_jpeg_compress_struct_prep(struct jpeg_compress_struct* u, void* v) { u->prep = (void*)v; }
void* lime_get_jpeg_compress_struct_coef(struct jpeg_compress_struct* u) { return (void*)u->coef; }
void lime_set_jpeg_compress_struct_coef(struct jpeg_compress_struct* u, void* v) { u->coef = (void*)v; }
void* lime_get_jpeg_compress_struct_marker(struct jpeg_compress_struct* u) { return (void*)u->marker; }
void lime_set_jpeg_compress_struct_marker(struct jpeg_compress_struct* u, void* v) { u->marker = (void*)v; }
void* lime_get_jpeg_compress_struct_cconvert(struct jpeg_compress_struct* u) { return (void*)u->cconvert; }
void lime_set_jpeg_compress_struct_cconvert(struct jpeg_compress_struct* u, void* v) { u->cconvert = (void*)v; }
void* lime_get_jpeg_compress_struct_downsample(struct jpeg_compress_struct* u) { return (void*)u->downsample; }
void lime_set_jpeg_compress_struct_downsample(struct jpeg_compress_struct* u, void* v) { u->downsample = (void*)v; }
void* lime_get_jpeg_compress_struct_fdct(struct jpeg_compress_struct* u) { return (void*)u->fdct; }
void lime_set_jpeg_compress_struct_fdct(struct jpeg_compress_struct* u, void* v) { u->fdct = (void*)v; }
void* lime_get_jpeg_compress_struct_entropy(struct jpeg_compress_struct* u) { return (void*)u->entropy; }
void lime_set_jpeg_compress_struct_entropy(struct jpeg_compress_struct* u, void* v) { u->entropy = (void*)v; }
void* lime_get_jpeg_compress_struct_script_space(struct jpeg_compress_struct* u) { return (void*)u->script_space; }
void lime_set_jpeg_compress_struct_script_space(struct jpeg_compress_struct* u, void* v) { u->script_space = (void*)v; }
int lime_get_jpeg_compress_struct_script_space_size(struct jpeg_compress_struct* u) { return (int)u->script_space_size; }
void lime_set_jpeg_compress_struct_script_space_size(struct jpeg_compress_struct* u, int v) { u->script_space_size = (int)v; }

void* lime_make_jpeg_decompress_struct(void) { return (void*)calloc(1, sizeof(struct jpeg_decompress_struct)); }
void* lime_get_jpeg_decompress_struct_err(struct jpeg_decompress_struct* u) { return (void*)u->err; }
void lime_set_jpeg_decompress_struct_err(struct jpeg_decompress_struct* u, void* v) { u->err = (void*)v; }
void* lime_get_jpeg_decompress_struct_mem(struct jpeg_decompress_struct* u) { return (void*)u->mem; }
void lime_set_jpeg_decompress_struct_mem(struct jpeg_decompress_struct* u, void* v) { u->mem = (void*)v; }
void* lime_get_jpeg_decompress_struct_progress(struct jpeg_decompress_struct* u) { return (void*)u->progress; }
void lime_set_jpeg_decompress_struct_progress(struct jpeg_decompress_struct* u, void* v) { u->progress = (void*)v; }
void* lime_get_jpeg_decompress_struct_client_data(struct jpeg_decompress_struct* u) { return (void*)u->client_data; }
void lime_set_jpeg_decompress_struct_client_data(struct jpeg_decompress_struct* u, void* v) { u->client_data = (void*)v; }
unsigned char lime_get_jpeg_decompress_struct_is_decompressor(struct jpeg_decompress_struct* u) { return (unsigned char)u->is_decompressor; }
void lime_set_jpeg_decompress_struct_is_decompressor(struct jpeg_decompress_struct* u, unsigned char v) { u->is_decompressor = (unsigned char)v; }
int lime_get_jpeg_decompress_struct_global_state(struct jpeg_decompress_struct* u) { return (int)u->global_state; }
void lime_set_jpeg_decompress_struct_global_state(struct jpeg_decompress_struct* u, int v) { u->global_state = (int)v; }
void* lime_get_jpeg_decompress_struct_src(struct jpeg_decompress_struct* u) { return (void*)u->src; }
void lime_set_jpeg_decompress_struct_src(struct jpeg_decompress_struct* u, void* v) { u->src = (void*)v; }
int lime_get_jpeg_decompress_struct_image_width(struct jpeg_decompress_struct* u) { return (int)u->image_width; }
void lime_set_jpeg_decompress_struct_image_width(struct jpeg_decompress_struct* u, int v) { u->image_width = (int)v; }
int lime_get_jpeg_decompress_struct_image_height(struct jpeg_decompress_struct* u) { return (int)u->image_height; }
void lime_set_jpeg_decompress_struct_image_height(struct jpeg_decompress_struct* u, int v) { u->image_height = (int)v; }
int lime_get_jpeg_decompress_struct_num_components(struct jpeg_decompress_struct* u) { return (int)u->num_components; }
void lime_set_jpeg_decompress_struct_num_components(struct jpeg_decompress_struct* u, int v) { u->num_components = (int)v; }
int lime_get_jpeg_decompress_struct_jpeg_color_space(struct jpeg_decompress_struct* u) { return (int)u->jpeg_color_space; }
void lime_set_jpeg_decompress_struct_jpeg_color_space(struct jpeg_decompress_struct* u, int v) { u->jpeg_color_space = (int)v; }
int lime_get_jpeg_decompress_struct_out_color_space(struct jpeg_decompress_struct* u) { return (int)u->out_color_space; }
void lime_set_jpeg_decompress_struct_out_color_space(struct jpeg_decompress_struct* u, int v) { u->out_color_space = (int)v; }
int lime_get_jpeg_decompress_struct_scale_num(struct jpeg_decompress_struct* u) { return (int)u->scale_num; }
void lime_set_jpeg_decompress_struct_scale_num(struct jpeg_decompress_struct* u, int v) { u->scale_num = (int)v; }
int lime_get_jpeg_decompress_struct_scale_denom(struct jpeg_decompress_struct* u) { return (int)u->scale_denom; }
void lime_set_jpeg_decompress_struct_scale_denom(struct jpeg_decompress_struct* u, int v) { u->scale_denom = (int)v; }
double lime_get_jpeg_decompress_struct_output_gamma(struct jpeg_decompress_struct* u) { return (double)u->output_gamma; }
void lime_set_jpeg_decompress_struct_output_gamma(struct jpeg_decompress_struct* u, double v) { u->output_gamma = (double)v; }
unsigned char lime_get_jpeg_decompress_struct_buffered_image(struct jpeg_decompress_struct* u) { return (unsigned char)u->buffered_image; }
void lime_set_jpeg_decompress_struct_buffered_image(struct jpeg_decompress_struct* u, unsigned char v) { u->buffered_image = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_raw_data_out(struct jpeg_decompress_struct* u) { return (unsigned char)u->raw_data_out; }
void lime_set_jpeg_decompress_struct_raw_data_out(struct jpeg_decompress_struct* u, unsigned char v) { u->raw_data_out = (unsigned char)v; }
int lime_get_jpeg_decompress_struct_dct_method(struct jpeg_decompress_struct* u) { return (int)u->dct_method; }
void lime_set_jpeg_decompress_struct_dct_method(struct jpeg_decompress_struct* u, int v) { u->dct_method = (int)v; }
unsigned char lime_get_jpeg_decompress_struct_do_fancy_upsampling(struct jpeg_decompress_struct* u) { return (unsigned char)u->do_fancy_upsampling; }
void lime_set_jpeg_decompress_struct_do_fancy_upsampling(struct jpeg_decompress_struct* u, unsigned char v) { u->do_fancy_upsampling = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_do_block_smoothing(struct jpeg_decompress_struct* u) { return (unsigned char)u->do_block_smoothing; }
void lime_set_jpeg_decompress_struct_do_block_smoothing(struct jpeg_decompress_struct* u, unsigned char v) { u->do_block_smoothing = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_quantize_colors(struct jpeg_decompress_struct* u) { return (unsigned char)u->quantize_colors; }
void lime_set_jpeg_decompress_struct_quantize_colors(struct jpeg_decompress_struct* u, unsigned char v) { u->quantize_colors = (unsigned char)v; }
int lime_get_jpeg_decompress_struct_dither_mode(struct jpeg_decompress_struct* u) { return (int)u->dither_mode; }
void lime_set_jpeg_decompress_struct_dither_mode(struct jpeg_decompress_struct* u, int v) { u->dither_mode = (int)v; }
unsigned char lime_get_jpeg_decompress_struct_two_pass_quantize(struct jpeg_decompress_struct* u) { return (unsigned char)u->two_pass_quantize; }
void lime_set_jpeg_decompress_struct_two_pass_quantize(struct jpeg_decompress_struct* u, unsigned char v) { u->two_pass_quantize = (unsigned char)v; }
int lime_get_jpeg_decompress_struct_desired_number_of_colors(struct jpeg_decompress_struct* u) { return (int)u->desired_number_of_colors; }
void lime_set_jpeg_decompress_struct_desired_number_of_colors(struct jpeg_decompress_struct* u, int v) { u->desired_number_of_colors = (int)v; }
unsigned char lime_get_jpeg_decompress_struct_enable_1pass_quant(struct jpeg_decompress_struct* u) { return (unsigned char)u->enable_1pass_quant; }
void lime_set_jpeg_decompress_struct_enable_1pass_quant(struct jpeg_decompress_struct* u, unsigned char v) { u->enable_1pass_quant = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_enable_external_quant(struct jpeg_decompress_struct* u) { return (unsigned char)u->enable_external_quant; }
void lime_set_jpeg_decompress_struct_enable_external_quant(struct jpeg_decompress_struct* u, unsigned char v) { u->enable_external_quant = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_enable_2pass_quant(struct jpeg_decompress_struct* u) { return (unsigned char)u->enable_2pass_quant; }
void lime_set_jpeg_decompress_struct_enable_2pass_quant(struct jpeg_decompress_struct* u, unsigned char v) { u->enable_2pass_quant = (unsigned char)v; }
int lime_get_jpeg_decompress_struct_output_width(struct jpeg_decompress_struct* u) { return (int)u->output_width; }
void lime_set_jpeg_decompress_struct_output_width(struct jpeg_decompress_struct* u, int v) { u->output_width = (int)v; }
int lime_get_jpeg_decompress_struct_output_height(struct jpeg_decompress_struct* u) { return (int)u->output_height; }
void lime_set_jpeg_decompress_struct_output_height(struct jpeg_decompress_struct* u, int v) { u->output_height = (int)v; }
int lime_get_jpeg_decompress_struct_out_color_components(struct jpeg_decompress_struct* u) { return (int)u->out_color_components; }
void lime_set_jpeg_decompress_struct_out_color_components(struct jpeg_decompress_struct* u, int v) { u->out_color_components = (int)v; }
int lime_get_jpeg_decompress_struct_output_components(struct jpeg_decompress_struct* u) { return (int)u->output_components; }
void lime_set_jpeg_decompress_struct_output_components(struct jpeg_decompress_struct* u, int v) { u->output_components = (int)v; }
int lime_get_jpeg_decompress_struct_rec_outbuf_height(struct jpeg_decompress_struct* u) { return (int)u->rec_outbuf_height; }
void lime_set_jpeg_decompress_struct_rec_outbuf_height(struct jpeg_decompress_struct* u, int v) { u->rec_outbuf_height = (int)v; }
int lime_get_jpeg_decompress_struct_actual_number_of_colors(struct jpeg_decompress_struct* u) { return (int)u->actual_number_of_colors; }
void lime_set_jpeg_decompress_struct_actual_number_of_colors(struct jpeg_decompress_struct* u, int v) { u->actual_number_of_colors = (int)v; }
void* lime_get_jpeg_decompress_struct_colormap(struct jpeg_decompress_struct* u) { return (void*)&u->colormap; }
void lime_set_jpeg_decompress_struct_colormap(struct jpeg_decompress_struct* u, void* v) { memcpy(&u->colormap, v, sizeof(u->colormap)); }
int lime_get_jpeg_decompress_struct_output_scanline(struct jpeg_decompress_struct* u) { return (int)u->output_scanline; }
void lime_set_jpeg_decompress_struct_output_scanline(struct jpeg_decompress_struct* u, int v) { u->output_scanline = (int)v; }
int lime_get_jpeg_decompress_struct_input_scan_number(struct jpeg_decompress_struct* u) { return (int)u->input_scan_number; }
void lime_set_jpeg_decompress_struct_input_scan_number(struct jpeg_decompress_struct* u, int v) { u->input_scan_number = (int)v; }
int lime_get_jpeg_decompress_struct_input_iMCU_row(struct jpeg_decompress_struct* u) { return (int)u->input_iMCU_row; }
void lime_set_jpeg_decompress_struct_input_iMCU_row(struct jpeg_decompress_struct* u, int v) { u->input_iMCU_row = (int)v; }
int lime_get_jpeg_decompress_struct_output_scan_number(struct jpeg_decompress_struct* u) { return (int)u->output_scan_number; }
void lime_set_jpeg_decompress_struct_output_scan_number(struct jpeg_decompress_struct* u, int v) { u->output_scan_number = (int)v; }
int lime_get_jpeg_decompress_struct_output_iMCU_row(struct jpeg_decompress_struct* u) { return (int)u->output_iMCU_row; }
void lime_set_jpeg_decompress_struct_output_iMCU_row(struct jpeg_decompress_struct* u, int v) { u->output_iMCU_row = (int)v; }
struct JQUANT_TBL* lime_get_jpeg_decompress_struct_quant_tbl_ptrs_i(struct jpeg_decompress_struct* u, int i) { return (struct JQUANT_TBL*)u->quant_tbl_ptrs[i]; }
void lime_set_jpeg_decompress_struct_quant_tbl_ptrs_i(struct jpeg_decompress_struct* u, int i, struct JQUANT_TBL* v) { u->quant_tbl_ptrs[i] = (struct JQUANT_TBL*)v; }
struct JHUFF_TBL* lime_get_jpeg_decompress_struct_dc_huff_tbl_ptrs_i(struct jpeg_decompress_struct* u, int i) { return (struct JHUFF_TBL*)u->dc_huff_tbl_ptrs[i]; }
void lime_set_jpeg_decompress_struct_dc_huff_tbl_ptrs_i(struct jpeg_decompress_struct* u, int i, struct JHUFF_TBL* v) { u->dc_huff_tbl_ptrs[i] = (struct JHUFF_TBL*)v; }
struct JHUFF_TBL* lime_get_jpeg_decompress_struct_ac_huff_tbl_ptrs_i(struct jpeg_decompress_struct* u, int i) { return (struct JHUFF_TBL*)u->ac_huff_tbl_ptrs[i]; }
void lime_set_jpeg_decompress_struct_ac_huff_tbl_ptrs_i(struct jpeg_decompress_struct* u, int i, struct JHUFF_TBL* v) { u->ac_huff_tbl_ptrs[i] = (struct JHUFF_TBL*)v; }
int lime_get_jpeg_decompress_struct_data_precision(struct jpeg_decompress_struct* u) { return (int)u->data_precision; }
void lime_set_jpeg_decompress_struct_data_precision(struct jpeg_decompress_struct* u, int v) { u->data_precision = (int)v; }
void* lime_get_jpeg_decompress_struct_comp_info(struct jpeg_decompress_struct* u) { return (void*)u->comp_info; }
void lime_set_jpeg_decompress_struct_comp_info(struct jpeg_decompress_struct* u, void* v) { u->comp_info = (void*)v; }
unsigned char lime_get_jpeg_decompress_struct_is_baseline(struct jpeg_decompress_struct* u) { return (unsigned char)u->is_baseline; }
void lime_set_jpeg_decompress_struct_is_baseline(struct jpeg_decompress_struct* u, unsigned char v) { u->is_baseline = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_progressive_mode(struct jpeg_decompress_struct* u) { return (unsigned char)u->progressive_mode; }
void lime_set_jpeg_decompress_struct_progressive_mode(struct jpeg_decompress_struct* u, unsigned char v) { u->progressive_mode = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_arith_code(struct jpeg_decompress_struct* u) { return (unsigned char)u->arith_code; }
void lime_set_jpeg_decompress_struct_arith_code(struct jpeg_decompress_struct* u, unsigned char v) { u->arith_code = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_arith_dc_L_i(struct jpeg_decompress_struct* u, int i) { return (unsigned char)u->arith_dc_L[i]; }
void lime_set_jpeg_decompress_struct_arith_dc_L_i(struct jpeg_decompress_struct* u, int i, unsigned char v) { u->arith_dc_L[i] = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_arith_dc_U_i(struct jpeg_decompress_struct* u, int i) { return (unsigned char)u->arith_dc_U[i]; }
void lime_set_jpeg_decompress_struct_arith_dc_U_i(struct jpeg_decompress_struct* u, int i, unsigned char v) { u->arith_dc_U[i] = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_arith_ac_K_i(struct jpeg_decompress_struct* u, int i) { return (unsigned char)u->arith_ac_K[i]; }
void lime_set_jpeg_decompress_struct_arith_ac_K_i(struct jpeg_decompress_struct* u, int i, unsigned char v) { u->arith_ac_K[i] = (unsigned char)v; }
int lime_get_jpeg_decompress_struct_restart_interval(struct jpeg_decompress_struct* u) { return (int)u->restart_interval; }
void lime_set_jpeg_decompress_struct_restart_interval(struct jpeg_decompress_struct* u, int v) { u->restart_interval = (int)v; }
unsigned char lime_get_jpeg_decompress_struct_saw_JFIF_marker(struct jpeg_decompress_struct* u) { return (unsigned char)u->saw_JFIF_marker; }
void lime_set_jpeg_decompress_struct_saw_JFIF_marker(struct jpeg_decompress_struct* u, unsigned char v) { u->saw_JFIF_marker = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_JFIF_major_version(struct jpeg_decompress_struct* u) { return (unsigned char)u->JFIF_major_version; }
void lime_set_jpeg_decompress_struct_JFIF_major_version(struct jpeg_decompress_struct* u, unsigned char v) { u->JFIF_major_version = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_JFIF_minor_version(struct jpeg_decompress_struct* u) { return (unsigned char)u->JFIF_minor_version; }
void lime_set_jpeg_decompress_struct_JFIF_minor_version(struct jpeg_decompress_struct* u, unsigned char v) { u->JFIF_minor_version = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_density_unit(struct jpeg_decompress_struct* u) { return (unsigned char)u->density_unit; }
void lime_set_jpeg_decompress_struct_density_unit(struct jpeg_decompress_struct* u, unsigned char v) { u->density_unit = (unsigned char)v; }
unsigned short lime_get_jpeg_decompress_struct_X_density(struct jpeg_decompress_struct* u) { return (unsigned short)u->X_density; }
void lime_set_jpeg_decompress_struct_X_density(struct jpeg_decompress_struct* u, unsigned short v) { u->X_density = (unsigned short)v; }
unsigned short lime_get_jpeg_decompress_struct_Y_density(struct jpeg_decompress_struct* u) { return (unsigned short)u->Y_density; }
void lime_set_jpeg_decompress_struct_Y_density(struct jpeg_decompress_struct* u, unsigned short v) { u->Y_density = (unsigned short)v; }
unsigned char lime_get_jpeg_decompress_struct_saw_Adobe_marker(struct jpeg_decompress_struct* u) { return (unsigned char)u->saw_Adobe_marker; }
void lime_set_jpeg_decompress_struct_saw_Adobe_marker(struct jpeg_decompress_struct* u, unsigned char v) { u->saw_Adobe_marker = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_Adobe_transform(struct jpeg_decompress_struct* u) { return (unsigned char)u->Adobe_transform; }
void lime_set_jpeg_decompress_struct_Adobe_transform(struct jpeg_decompress_struct* u, unsigned char v) { u->Adobe_transform = (unsigned char)v; }
unsigned char lime_get_jpeg_decompress_struct_CCIR601_sampling(struct jpeg_decompress_struct* u) { return (unsigned char)u->CCIR601_sampling; }
void lime_set_jpeg_decompress_struct_CCIR601_sampling(struct jpeg_decompress_struct* u, unsigned char v) { u->CCIR601_sampling = (unsigned char)v; }
void* lime_get_jpeg_decompress_struct_marker_list(struct jpeg_decompress_struct* u) { return (void*)&u->marker_list; }
void lime_set_jpeg_decompress_struct_marker_list(struct jpeg_decompress_struct* u, void* v) { memcpy(&u->marker_list, v, sizeof(u->marker_list)); }
int lime_get_jpeg_decompress_struct_max_h_samp_factor(struct jpeg_decompress_struct* u) { return (int)u->max_h_samp_factor; }
void lime_set_jpeg_decompress_struct_max_h_samp_factor(struct jpeg_decompress_struct* u, int v) { u->max_h_samp_factor = (int)v; }
int lime_get_jpeg_decompress_struct_max_v_samp_factor(struct jpeg_decompress_struct* u) { return (int)u->max_v_samp_factor; }
void lime_set_jpeg_decompress_struct_max_v_samp_factor(struct jpeg_decompress_struct* u, int v) { u->max_v_samp_factor = (int)v; }
int lime_get_jpeg_decompress_struct_min_DCT_h_scaled_size(struct jpeg_decompress_struct* u) { return (int)u->min_DCT_h_scaled_size; }
void lime_set_jpeg_decompress_struct_min_DCT_h_scaled_size(struct jpeg_decompress_struct* u, int v) { u->min_DCT_h_scaled_size = (int)v; }
int lime_get_jpeg_decompress_struct_min_DCT_v_scaled_size(struct jpeg_decompress_struct* u) { return (int)u->min_DCT_v_scaled_size; }
void lime_set_jpeg_decompress_struct_min_DCT_v_scaled_size(struct jpeg_decompress_struct* u, int v) { u->min_DCT_v_scaled_size = (int)v; }
int lime_get_jpeg_decompress_struct_total_iMCU_rows(struct jpeg_decompress_struct* u) { return (int)u->total_iMCU_rows; }
void lime_set_jpeg_decompress_struct_total_iMCU_rows(struct jpeg_decompress_struct* u, int v) { u->total_iMCU_rows = (int)v; }
void* lime_get_jpeg_decompress_struct_sample_range_limit(struct jpeg_decompress_struct* u) { return (void*)u->sample_range_limit; }
void lime_set_jpeg_decompress_struct_sample_range_limit(struct jpeg_decompress_struct* u, void* v) { u->sample_range_limit = (void*)v; }
int lime_get_jpeg_decompress_struct_comps_in_scan(struct jpeg_decompress_struct* u) { return (int)u->comps_in_scan; }
void lime_set_jpeg_decompress_struct_comps_in_scan(struct jpeg_decompress_struct* u, int v) { u->comps_in_scan = (int)v; }
struct jpeg_component_info* lime_get_jpeg_decompress_struct_cur_comp_info_i(struct jpeg_decompress_struct* u, int i) { return (struct jpeg_component_info*)u->cur_comp_info[i]; }
void lime_set_jpeg_decompress_struct_cur_comp_info_i(struct jpeg_decompress_struct* u, int i, struct jpeg_component_info* v) { u->cur_comp_info[i] = (struct jpeg_component_info*)v; }
int lime_get_jpeg_decompress_struct_MCUs_per_row(struct jpeg_decompress_struct* u) { return (int)u->MCUs_per_row; }
void lime_set_jpeg_decompress_struct_MCUs_per_row(struct jpeg_decompress_struct* u, int v) { u->MCUs_per_row = (int)v; }
int lime_get_jpeg_decompress_struct_MCU_rows_in_scan(struct jpeg_decompress_struct* u) { return (int)u->MCU_rows_in_scan; }
void lime_set_jpeg_decompress_struct_MCU_rows_in_scan(struct jpeg_decompress_struct* u, int v) { u->MCU_rows_in_scan = (int)v; }
int lime_get_jpeg_decompress_struct_blocks_in_MCU(struct jpeg_decompress_struct* u) { return (int)u->blocks_in_MCU; }
void lime_set_jpeg_decompress_struct_blocks_in_MCU(struct jpeg_decompress_struct* u, int v) { u->blocks_in_MCU = (int)v; }
int lime_get_jpeg_decompress_struct_MCU_membership_i(struct jpeg_decompress_struct* u, int i) { return (int)u->MCU_membership[i]; }
void lime_set_jpeg_decompress_struct_MCU_membership_i(struct jpeg_decompress_struct* u, int i, int v) { u->MCU_membership[i] = (int)v; }
int lime_get_jpeg_decompress_struct_Ss(struct jpeg_decompress_struct* u) { return (int)u->Ss; }
void lime_set_jpeg_decompress_struct_Ss(struct jpeg_decompress_struct* u, int v) { u->Ss = (int)v; }
int lime_get_jpeg_decompress_struct_Se(struct jpeg_decompress_struct* u) { return (int)u->Se; }
void lime_set_jpeg_decompress_struct_Se(struct jpeg_decompress_struct* u, int v) { u->Se = (int)v; }
int lime_get_jpeg_decompress_struct_Ah(struct jpeg_decompress_struct* u) { return (int)u->Ah; }
void lime_set_jpeg_decompress_struct_Ah(struct jpeg_decompress_struct* u, int v) { u->Ah = (int)v; }
int lime_get_jpeg_decompress_struct_Al(struct jpeg_decompress_struct* u) { return (int)u->Al; }
void lime_set_jpeg_decompress_struct_Al(struct jpeg_decompress_struct* u, int v) { u->Al = (int)v; }
int lime_get_jpeg_decompress_struct_block_size(struct jpeg_decompress_struct* u) { return (int)u->block_size; }
void lime_set_jpeg_decompress_struct_block_size(struct jpeg_decompress_struct* u, int v) { u->block_size = (int)v; }
void* lime_get_jpeg_decompress_struct_natural_order(struct jpeg_decompress_struct* u) { return (void*)u->natural_order; }
void lime_set_jpeg_decompress_struct_natural_order(struct jpeg_decompress_struct* u, void* v) { u->natural_order = (void*)v; }
int lime_get_jpeg_decompress_struct_lim_Se(struct jpeg_decompress_struct* u) { return (int)u->lim_Se; }
void lime_set_jpeg_decompress_struct_lim_Se(struct jpeg_decompress_struct* u, int v) { u->lim_Se = (int)v; }
int lime_get_jpeg_decompress_struct_unread_marker(struct jpeg_decompress_struct* u) { return (int)u->unread_marker; }
void lime_set_jpeg_decompress_struct_unread_marker(struct jpeg_decompress_struct* u, int v) { u->unread_marker = (int)v; }
void* lime_get_jpeg_decompress_struct_master(struct jpeg_decompress_struct* u) { return (void*)u->master; }
void lime_set_jpeg_decompress_struct_master(struct jpeg_decompress_struct* u, void* v) { u->master = (void*)v; }
void* lime_get_jpeg_decompress_struct_main(struct jpeg_decompress_struct* u) { return (void*)u->main; }
void lime_set_jpeg_decompress_struct_main(struct jpeg_decompress_struct* u, void* v) { u->main = (void*)v; }
void* lime_get_jpeg_decompress_struct_coef(struct jpeg_decompress_struct* u) { return (void*)u->coef; }
void lime_set_jpeg_decompress_struct_coef(struct jpeg_decompress_struct* u, void* v) { u->coef = (void*)v; }
void* lime_get_jpeg_decompress_struct_post(struct jpeg_decompress_struct* u) { return (void*)u->post; }
void lime_set_jpeg_decompress_struct_post(struct jpeg_decompress_struct* u, void* v) { u->post = (void*)v; }
void* lime_get_jpeg_decompress_struct_inputctl(struct jpeg_decompress_struct* u) { return (void*)u->inputctl; }
void lime_set_jpeg_decompress_struct_inputctl(struct jpeg_decompress_struct* u, void* v) { u->inputctl = (void*)v; }
void* lime_get_jpeg_decompress_struct_marker(struct jpeg_decompress_struct* u) { return (void*)u->marker; }
void lime_set_jpeg_decompress_struct_marker(struct jpeg_decompress_struct* u, void* v) { u->marker = (void*)v; }
void* lime_get_jpeg_decompress_struct_entropy(struct jpeg_decompress_struct* u) { return (void*)u->entropy; }
void lime_set_jpeg_decompress_struct_entropy(struct jpeg_decompress_struct* u, void* v) { u->entropy = (void*)v; }
void* lime_get_jpeg_decompress_struct_idct(struct jpeg_decompress_struct* u) { return (void*)u->idct; }
void lime_set_jpeg_decompress_struct_idct(struct jpeg_decompress_struct* u, void* v) { u->idct = (void*)v; }
void* lime_get_jpeg_decompress_struct_upsample(struct jpeg_decompress_struct* u) { return (void*)u->upsample; }
void lime_set_jpeg_decompress_struct_upsample(struct jpeg_decompress_struct* u, void* v) { u->upsample = (void*)v; }
void* lime_get_jpeg_decompress_struct_cconvert(struct jpeg_decompress_struct* u) { return (void*)u->cconvert; }
void lime_set_jpeg_decompress_struct_cconvert(struct jpeg_decompress_struct* u, void* v) { u->cconvert = (void*)v; }
void* lime_get_jpeg_decompress_struct_cquantize(struct jpeg_decompress_struct* u) { return (void*)u->cquantize; }
void lime_set_jpeg_decompress_struct_cquantize(struct jpeg_decompress_struct* u, void* v) { u->cquantize = (void*)v; }

void* lime_make_jpeg_error_mgr(void) { return (void*)calloc(1, sizeof(struct jpeg_error_mgr)); }
int lime_get_jpeg_error_mgr_msg_code(struct jpeg_error_mgr* u) { return (int)u->msg_code; }
void lime_set_jpeg_error_mgr_msg_code(struct jpeg_error_mgr* u, int v) { u->msg_code = (int)v; }
int lime_get_jpeg_error_mgr_trace_level(struct jpeg_error_mgr* u) { return (int)u->trace_level; }
void lime_set_jpeg_error_mgr_trace_level(struct jpeg_error_mgr* u, int v) { u->trace_level = (int)v; }
long long lime_get_jpeg_error_mgr_num_warnings(struct jpeg_error_mgr* u) { return (long long)u->num_warnings; }
void lime_set_jpeg_error_mgr_num_warnings(struct jpeg_error_mgr* u, long long v) { u->num_warnings = (long long)v; }
void* lime_get_jpeg_error_mgr_jpeg_message_table(struct jpeg_error_mgr* u) { return (void*)u->jpeg_message_table; }
void lime_set_jpeg_error_mgr_jpeg_message_table(struct jpeg_error_mgr* u, void* v) { u->jpeg_message_table = (void*)v; }
int lime_get_jpeg_error_mgr_last_jpeg_message(struct jpeg_error_mgr* u) { return (int)u->last_jpeg_message; }
void lime_set_jpeg_error_mgr_last_jpeg_message(struct jpeg_error_mgr* u, int v) { u->last_jpeg_message = (int)v; }
void* lime_get_jpeg_error_mgr_addon_message_table(struct jpeg_error_mgr* u) { return (void*)u->addon_message_table; }
void lime_set_jpeg_error_mgr_addon_message_table(struct jpeg_error_mgr* u, void* v) { u->addon_message_table = (void*)v; }
int lime_get_jpeg_error_mgr_first_addon_message(struct jpeg_error_mgr* u) { return (int)u->first_addon_message; }
void lime_set_jpeg_error_mgr_first_addon_message(struct jpeg_error_mgr* u, int v) { u->first_addon_message = (int)v; }
int lime_get_jpeg_error_mgr_last_addon_message(struct jpeg_error_mgr* u) { return (int)u->last_addon_message; }
void lime_set_jpeg_error_mgr_last_addon_message(struct jpeg_error_mgr* u, int v) { u->last_addon_message = (int)v; }
char lime_get_jpeg_error_mgr_s_i(struct jpeg_error_mgr* u, int i) { return (char)u->s[i]; }
void lime_set_jpeg_error_mgr_s_i(struct jpeg_error_mgr* u, int i, char v) { u->s[i] = (char)v; }

void lime_set_jpeg_error_mgr_error_exit(struct jpeg_error_mgr* t, void* f) { *(void**)(&t->error_exit) = f; }
void lime_set_jpeg_error_mgr_error_exit_null(struct jpeg_error_mgr* t) { t->error_exit = 0; }
void lime_set_jpeg_error_mgr_emit_message(struct jpeg_error_mgr* t, void* f) { *(void**)(&t->emit_message) = f; }
void lime_set_jpeg_error_mgr_emit_message_null(struct jpeg_error_mgr* t) { t->emit_message = 0; }
void lime_set_jpeg_error_mgr_output_message(struct jpeg_error_mgr* t, void* f) { *(void**)(&t->output_message) = f; }
void lime_set_jpeg_error_mgr_output_message_null(struct jpeg_error_mgr* t) { t->output_message = 0; }
void lime_set_jpeg_error_mgr_format_message(struct jpeg_error_mgr* t, void* f) { *(void**)(&t->format_message) = f; }
void lime_set_jpeg_error_mgr_format_message_null(struct jpeg_error_mgr* t) { t->format_message = 0; }
void lime_set_jpeg_error_mgr_reset_error_mgr(struct jpeg_error_mgr* t, void* f) { *(void**)(&t->reset_error_mgr) = f; }
void lime_set_jpeg_error_mgr_reset_error_mgr_null(struct jpeg_error_mgr* t) { t->reset_error_mgr = 0; }

void* lime_make_jpeg_progress_mgr(void) { return (void*)calloc(1, sizeof(struct jpeg_progress_mgr)); }
long long lime_get_jpeg_progress_mgr_pass_counter(struct jpeg_progress_mgr* u) { return (long long)u->pass_counter; }
void lime_set_jpeg_progress_mgr_pass_counter(struct jpeg_progress_mgr* u, long long v) { u->pass_counter = (long long)v; }
long long lime_get_jpeg_progress_mgr_pass_limit(struct jpeg_progress_mgr* u) { return (long long)u->pass_limit; }
void lime_set_jpeg_progress_mgr_pass_limit(struct jpeg_progress_mgr* u, long long v) { u->pass_limit = (long long)v; }
int lime_get_jpeg_progress_mgr_completed_passes(struct jpeg_progress_mgr* u) { return (int)u->completed_passes; }
void lime_set_jpeg_progress_mgr_completed_passes(struct jpeg_progress_mgr* u, int v) { u->completed_passes = (int)v; }
int lime_get_jpeg_progress_mgr_total_passes(struct jpeg_progress_mgr* u) { return (int)u->total_passes; }
void lime_set_jpeg_progress_mgr_total_passes(struct jpeg_progress_mgr* u, int v) { u->total_passes = (int)v; }

void lime_set_jpeg_progress_mgr_progress_monitor(struct jpeg_progress_mgr* t, void* f) { *(void**)(&t->progress_monitor) = f; }
void lime_set_jpeg_progress_mgr_progress_monitor_null(struct jpeg_progress_mgr* t) { t->progress_monitor = 0; }

void* lime_make_jpeg_destination_mgr(void) { return (void*)calloc(1, sizeof(struct jpeg_destination_mgr)); }
void* lime_get_jpeg_destination_mgr_next_output_byte(struct jpeg_destination_mgr* u) { return (void*)u->next_output_byte; }
void lime_set_jpeg_destination_mgr_next_output_byte(struct jpeg_destination_mgr* u, void* v) { u->next_output_byte = (void*)v; }
size_t lime_get_jpeg_destination_mgr_free_in_buffer(struct jpeg_destination_mgr* u) { return (size_t)u->free_in_buffer; }
void lime_set_jpeg_destination_mgr_free_in_buffer(struct jpeg_destination_mgr* u, size_t v) { u->free_in_buffer = (size_t)v; }

void lime_set_jpeg_destination_mgr_init_destination(struct jpeg_destination_mgr* t, void* f) { *(void**)(&t->init_destination) = f; }
void lime_set_jpeg_destination_mgr_init_destination_null(struct jpeg_destination_mgr* t) { t->init_destination = 0; }
void lime_set_jpeg_destination_mgr_empty_output_buffer(struct jpeg_destination_mgr* t, void* f) { *(void**)(&t->empty_output_buffer) = f; }
void lime_set_jpeg_destination_mgr_empty_output_buffer_null(struct jpeg_destination_mgr* t) { t->empty_output_buffer = 0; }
void lime_set_jpeg_destination_mgr_term_destination(struct jpeg_destination_mgr* t, void* f) { *(void**)(&t->term_destination) = f; }
void lime_set_jpeg_destination_mgr_term_destination_null(struct jpeg_destination_mgr* t) { t->term_destination = 0; }

void* lime_make_jpeg_source_mgr(void) { return (void*)calloc(1, sizeof(struct jpeg_source_mgr)); }
void* lime_get_jpeg_source_mgr_next_input_byte(struct jpeg_source_mgr* u) { return (void*)u->next_input_byte; }
void lime_set_jpeg_source_mgr_next_input_byte(struct jpeg_source_mgr* u, void* v) { u->next_input_byte = (void*)v; }
size_t lime_get_jpeg_source_mgr_bytes_in_buffer(struct jpeg_source_mgr* u) { return (size_t)u->bytes_in_buffer; }
void lime_set_jpeg_source_mgr_bytes_in_buffer(struct jpeg_source_mgr* u, size_t v) { u->bytes_in_buffer = (size_t)v; }

void lime_set_jpeg_source_mgr_init_source(struct jpeg_source_mgr* t, void* f) { *(void**)(&t->init_source) = f; }
void lime_set_jpeg_source_mgr_init_source_null(struct jpeg_source_mgr* t) { t->init_source = 0; }
void lime_set_jpeg_source_mgr_fill_input_buffer(struct jpeg_source_mgr* t, void* f) { *(void**)(&t->fill_input_buffer) = f; }
void lime_set_jpeg_source_mgr_fill_input_buffer_null(struct jpeg_source_mgr* t) { t->fill_input_buffer = 0; }
void lime_set_jpeg_source_mgr_skip_input_data(struct jpeg_source_mgr* t, void* f) { *(void**)(&t->skip_input_data) = f; }
void lime_set_jpeg_source_mgr_skip_input_data_null(struct jpeg_source_mgr* t) { t->skip_input_data = 0; }
void lime_set_jpeg_source_mgr_resync_to_restart(struct jpeg_source_mgr* t, void* f) { *(void**)(&t->resync_to_restart) = f; }
void lime_set_jpeg_source_mgr_resync_to_restart_null(struct jpeg_source_mgr* t) { t->resync_to_restart = 0; }
void lime_set_jpeg_source_mgr_term_source(struct jpeg_source_mgr* t, void* f) { *(void**)(&t->term_source) = f; }
void lime_set_jpeg_source_mgr_term_source_null(struct jpeg_source_mgr* t) { t->term_source = 0; }

void* lime_make_jpeg_memory_mgr(void) { return (void*)calloc(1, sizeof(struct jpeg_memory_mgr)); }
long long lime_get_jpeg_memory_mgr_max_memory_to_use(struct jpeg_memory_mgr* u) { return (long long)u->max_memory_to_use; }
void lime_set_jpeg_memory_mgr_max_memory_to_use(struct jpeg_memory_mgr* u, long long v) { u->max_memory_to_use = (long long)v; }
long long lime_get_jpeg_memory_mgr_max_alloc_chunk(struct jpeg_memory_mgr* u) { return (long long)u->max_alloc_chunk; }
void lime_set_jpeg_memory_mgr_max_alloc_chunk(struct jpeg_memory_mgr* u, long long v) { u->max_alloc_chunk = (long long)v; }

void lime_set_jpeg_memory_mgr_alloc_small(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->alloc_small) = f; }
void lime_set_jpeg_memory_mgr_alloc_small_null(struct jpeg_memory_mgr* t) { t->alloc_small = 0; }
void lime_set_jpeg_memory_mgr_alloc_large(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->alloc_large) = f; }
void lime_set_jpeg_memory_mgr_alloc_large_null(struct jpeg_memory_mgr* t) { t->alloc_large = 0; }
void lime_set_jpeg_memory_mgr_alloc_sarray(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->alloc_sarray) = f; }
void lime_set_jpeg_memory_mgr_alloc_sarray_null(struct jpeg_memory_mgr* t) { t->alloc_sarray = 0; }
void lime_set_jpeg_memory_mgr_alloc_barray(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->alloc_barray) = f; }
void lime_set_jpeg_memory_mgr_alloc_barray_null(struct jpeg_memory_mgr* t) { t->alloc_barray = 0; }
void lime_set_jpeg_memory_mgr_request_virt_sarray(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->request_virt_sarray) = f; }
void lime_set_jpeg_memory_mgr_request_virt_sarray_null(struct jpeg_memory_mgr* t) { t->request_virt_sarray = 0; }
void lime_set_jpeg_memory_mgr_request_virt_barray(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->request_virt_barray) = f; }
void lime_set_jpeg_memory_mgr_request_virt_barray_null(struct jpeg_memory_mgr* t) { t->request_virt_barray = 0; }
void lime_set_jpeg_memory_mgr_realize_virt_arrays(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->realize_virt_arrays) = f; }
void lime_set_jpeg_memory_mgr_realize_virt_arrays_null(struct jpeg_memory_mgr* t) { t->realize_virt_arrays = 0; }
void lime_set_jpeg_memory_mgr_access_virt_sarray(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->access_virt_sarray) = f; }
void lime_set_jpeg_memory_mgr_access_virt_sarray_null(struct jpeg_memory_mgr* t) { t->access_virt_sarray = 0; }
void lime_set_jpeg_memory_mgr_access_virt_barray(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->access_virt_barray) = f; }
void lime_set_jpeg_memory_mgr_access_virt_barray_null(struct jpeg_memory_mgr* t) { t->access_virt_barray = 0; }
void lime_set_jpeg_memory_mgr_free_pool(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->free_pool) = f; }
void lime_set_jpeg_memory_mgr_free_pool_null(struct jpeg_memory_mgr* t) { t->free_pool = 0; }
void lime_set_jpeg_memory_mgr_self_destruct(struct jpeg_memory_mgr* t, void* f) { *(void**)(&t->self_destruct) = f; }
void lime_set_jpeg_memory_mgr_self_destruct_null(struct jpeg_memory_mgr* t) { t->self_destruct = 0; }

int lime_const_JCS_EXTENSIONS(void) { return (int)(1); }

int lime_const_JCS_ALPHA_EXTENSIONS(void) { return (int)(1); }

int lime_const_JMSG_STR_PARM_MAX(void) { return (int)(80); }

int lime_const_JPOOL_NUMPOOLS(void) { return (int)(2); }

FILE* lime_out_freopen_s (char* a1, char* a2, FILE* a3) {
    FILE* a0 = 0;
    freopen_s(&a0, a1, a2, a3);
    return a0;
}

FILE* lime_out_tmpfile_s () {
    FILE* a0 = 0;
    tmpfile_s(&a0);
    return a0;
}

FILE* lime_out_fopen_s (char* a1, char* a2) {
    FILE* a0 = 0;
    fopen_s(&a0, a1, a2);
    return a0;
}

