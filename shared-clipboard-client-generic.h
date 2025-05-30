#pragma once

#include <stdint.h>
#include <stdbool.h>

typedef struct shared_clipboard_config_t shared_clipboard_config_t;

typedef struct {
  uint8_t *text;
  bool *connectivity;
} shared_clipboard_output_t;

void shared_clipboard_setup(void);

void shared_clipboard_start_thread(shared_clipboard_config_t *config);

bool shared_clipboard_stop_thread(void);

void shared_clipboard_send(const uint8_t *text);

void shared_clipboard_output_drop(shared_clipboard_output_t output);

shared_clipboard_output_t shared_clipboard_poll(void);

shared_clipboard_config_t *shared_clipboard_config_read_from_xdg_cofig_dir(void);

shared_clipboard_config_t *shared_clipboard_config_new(const uint8_t *url,
                                                       const uint8_t *token,
                                                       const uint8_t *name);
