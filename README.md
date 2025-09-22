# MPClipboard, shared and generic part

This is a shared part of all apps that implement MPClipboard's communication protocol.

It has both Rust and C APIs.

### Usage example

```c
// First, initialize the library
mpclipboard_init();

// Then make a config, in our case we read it from "./config.toml"
mpclipboard_config_read_option_t read_from = MPCLIPBOARD_CONFIG_READ_OPTION_T_FROM_LOCAL_FILE;
mpclipboard_config_t *config = mpclipboard_config_read(read_from);

// Start a background thread
mpclipboard_handle_t *handle = mpclipboard_thread_start(config);

// Do a bit of looping for data reading
for (int i = 0; i < 10; i++) {
    // Read the most recent state (clip + connectivity change)
    mpclipboard_output_t output = mpclipboard_handle_poll(handle);
    // Clip's text is NULLable
    if (output.text) {
        printf("text = %s\n", output.text);
        free(output.text);
    }
    // And connectivity change too
    if (output.connectivity) {
        printf("connectivity = %s\n", *output.connectivity ? "true" : "false");
        free(output.connectivity);
    }

    // Sleep a bit
    usleep(1000);
}

// Shutdown gracefully our background thread
mpclipboard_handle_stop(handle);
```

Rust API is similar and of course more Rust idiomatic (async and ADT-ish)
