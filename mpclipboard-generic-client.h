#pragma once

#include <stdint.h>
#include <stdbool.h>

/**
 * Instruction for the `Config::read` function how to read the config.
 */
typedef enum {
  /**
   * Read from "./config.toml", based on your current working directory
   */
  MPCLIPBOARD_CONFIG_READ_OPTION_T_FROM_LOCAL_FILE = 0,
  /**
   * Read from XDG Config dir (i.e. from `~/.config/mpclipboard/config.toml`)
   */
  MPCLIPBOARD_CONFIG_READ_OPTION_T_FROM_XDG_CONFIG_DIR = 1,
} mpclipboard_config_read_option_t;

/**
 * Representation of a runtime configuration
 */
typedef struct mpclipboard_config_t mpclipboard_config_t;

/**
 * Representation of a "handle" for running MPClipboard
 */
typedef struct mpclipboard_handle_t mpclipboard_handle_t;

/**
 * Represents a result of polling
 */
typedef struct {
  /**
   * Optional (NULLable) text received from the server
   */
  char *text;
  /**
   * Optional (NULLable) flag of the connectivity state
   */
  bool *connectivity;
} mpclipboard_output_t;

/**
 * Initializes MPClipboard's Logger and TLS connector.
 *
 * This is the first this that you must do before calling any
 * MPClipboard functions.
 *
 * Returns `false` if TLS connector can't be initialized.
 */
bool mpclipboard_init(void);

/**
 * Reads the config based on the given isntruction
 * (which is either "read from XDG dir" or "read from ./config.toml")
 */
mpclipboard_config_t *mpclipboard_config_read(mpclipboard_config_read_option_t option);

/**
 * Constrcuts the config in-place based on given parameters that match fields 1-to-1.
 */
mpclipboard_config_t *mpclipboard_config_new(const char *uri, const char *token, const char *name);

/**
 * Sends text from local clipboard, blocks until background thread receives
 * this text and decides whether it's a duplicate or not. Doesn't wait for delivery.
 * Returns `true` if given text is new (in such case it gets sent to the server).
 *
 * # Safety
 *
 * `handle` must be a valid pointer to Handle
 * `text` must be a NULL terminated C string
 */
bool mpclipboard_handle_send(const mpclipboard_handle_t *handle, const char *text);

/**
 * Polls background thread for any updates, squashes them and returns back to the caller.
 * Returns a pair of `new text received from the server` + `change of the connectivity`.
 * Both pair items can be empty (e.g. if there were to clips sent from the server)
 *
 * # Safety
 *
 * `handle` must be a valid pointer to Handle
 */
mpclipboard_output_t mpclipboard_handle_poll(mpclipboard_handle_t *handle);

/**
 * Gracefully shuts down a background thread
 *
 * # Safety
 *
 * `handle` must be a valid pointer to Handle
 */
bool mpclipboard_handle_stop(mpclipboard_handle_t *handle);

/**
 * Takes and returns a pipe reader that can be used to subscribe to updates
 * in poll/epoll -like fashion.
 * Every time there's an update this FD will get an update
 * and so you can `poll` it to know when to call `recv`.
 *
 * This way if you don't get any clips from the server you can stay in non-busy loop
 * and only `recv` when you know there's something to receive.
 *
 * # Safety
 *
 * `handle` must be a valid pointer to Handle
 */
int mpclipboard_handle_take_fd(mpclipboard_handle_t *handle);

/**
 * Prints one "info" and onr "error" message, useful for testing
 */
void mpclipboard_logger_test(void);

/**
 * Starts a background thread with Tokio runtime, returns a "handle" for communication and control.
 *
 * # Safety
 *
 * `config` must be a valid owned pointer to Config
 */
mpclipboard_handle_t *mpclipboard_thread_start(mpclipboard_config_t *config);
