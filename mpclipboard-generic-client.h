#pragma once

#include <stdint.h>
#include <stdbool.h>

typedef struct mpclipboard_config_t mpclipboard_config_t;

typedef struct {
  uint8_t *text;
  bool *connectivity;
} mpclipboard_output_t;

void mpclipboard_setup(void);

void mpclipboard_start_thread(mpclipboard_config_t *config);

bool mpclipboard_stop_thread(void);

void mpclipboard_send(const uint8_t *text);

mpclipboard_output_t mpclipboard_poll(void);

mpclipboard_config_t *mpclipboard_config_read_from_xdg_config_dir(void);

mpclipboard_config_t *mpclipboard_config_new(const uint8_t *url,
                                             const uint8_t *token,
                                             const uint8_t *name);
