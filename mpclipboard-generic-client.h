#pragma once

#include <stdint.h>
#include <stdbool.h>

typedef enum {
  MPCLIPBOARD_CONFIG_READ_OPTION_T_FROM_LOCAL_FILE = 0,
  MPCLIPBOARD_CONFIG_READ_OPTION_T_FROM_XDG_CONFIG_DIR = 1,
} mpclipboard_config_read_option_t;

typedef struct Clip Clip;

typedef struct mpclipboard_config_t mpclipboard_config_t;

typedef struct mpclipboard_handle_t mpclipboard_handle_t;

typedef struct Store Store;

typedef struct {
  Clip *clip;
  bool *connectivity;
} mpclipboard_output_t;

char *mpclipboard_clip_get_text(const Clip *clip);

void mpclipboard_clip_drop(Clip *clip);

mpclipboard_config_t *mpclipboard_config_read(mpclipboard_config_read_option_t option);

mpclipboard_config_t *mpclipboard_config_new(const char *uri, const char *token, const char *name);

void mpclipboard_handle_send(mpclipboard_handle_t *handle, const char *text);

mpclipboard_output_t mpclipboard_handle_poll(mpclipboard_handle_t *handle);

bool mpclipboard_handle_stop(mpclipboard_handle_t *handle);

int mpclipboard_handle_get_fd(const mpclipboard_handle_t *handle);

void mpclipboard_logger_init(void);

void mpclipboard_logger_test(void);

Store *mpclipboard_store_new(void);

void mpclipboard_store_drop(Store *store);

bool mpclipboard_store_add(Store *store, Clip *clip);

mpclipboard_handle_t *mpclipboard_thread_start(mpclipboard_config_t *config);

bool mpclipboard_tls_init(void);
