#pragma once

#include <stdint.h>
#include <stdbool.h>

typedef enum {
  MPCLIPBOARD_CONFIG_READ_OPTION_T_FROM_LOCAL_FILE = 0,
  MPCLIPBOARD_CONFIG_READ_OPTION_T_FROM_XDG_CONFIG_DIR = 1,
} mpclipboard_config_read_option_t;

typedef struct mpclipboard_config_t mpclipboard_config_t;

typedef struct mpclipboard_handle_t mpclipboard_handle_t;

typedef struct {
  uint8_t *text;
  bool *connectivity;
} mpclipboard_output_t;

bool mpclipboard_init(void);

mpclipboard_handle_t *mpclipboard_start_thread(mpclipboard_config_t *config);

bool mpclipboard_stop_thread(mpclipboard_handle_t *handle);

mpclipboard_config_t *mpclipboard_config_read(mpclipboard_config_read_option_t option);

mpclipboard_config_t *mpclipboard_config_new(const uint8_t *uri,
                                             const uint8_t *token,
                                             const uint8_t *name);

void mpclipboard_send(mpclipboard_handle_t *handle, const uint8_t *text);

mpclipboard_output_t mpclipboard_poll(mpclipboard_handle_t *handle);

void mpclipboard_test_logger(void);
