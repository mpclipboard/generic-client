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
  char *text;
  bool *connectivity;
} mpclipboard_output_t;

bool mpclipboard_init(void);

mpclipboard_config_t *mpclipboard_config_read(mpclipboard_config_read_option_t option);

mpclipboard_config_t *mpclipboard_config_new(const char *uri, const char *token, const char *name);

/**
 * # Safety
 *
 * `handle` must be a valid pointer to Handle
 * `text` must be a NULL terminated C string
 */
bool mpclipboard_handle_send(const mpclipboard_handle_t *handle, const char *text);

/**
 * # Safety
 *
 * `handle` must be a valid pointer to Handle
 */
mpclipboard_output_t mpclipboard_handle_poll(mpclipboard_handle_t *handle);

/**
 * # Safety
 *
 * `handle` must be a valid pointer to Handle
 */
bool mpclipboard_handle_stop(mpclipboard_handle_t *handle);

/**
 * # Safety
 *
 * `handle` must be a valid pointer to Handle
 */
int mpclipboard_handle_take_fd(mpclipboard_handle_t *handle);

void mpclipboard_logger_test(void);

/**
 * # Safety
 *
 * `config` must be a valid owned pointer to Config
 */
mpclipboard_handle_t *mpclipboard_thread_start(mpclipboard_config_t *config);
