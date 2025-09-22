#include "../mpclipboard-generic-client.h"
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

void *start_polling(void *handle);

int main() {
  mpclipboard_init();

  mpclipboard_config_t *config =
      mpclipboard_config_read(MPCLIPBOARD_CONFIG_READ_OPTION_T_FROM_LOCAL_FILE);

  mpclipboard_handle_t *handle = mpclipboard_thread_start(config);

  pthread_t thread;
  pthread_create(&thread, NULL, start_polling, handle);

  size_t line_length = 100;
  char *line = malloc(line_length);
  while (true) {
    getline(&line, &line_length, stdin);
    if (strcmp(line, "exit\n") == 0) {
      break;
    }
    if (mpclipboard_handle_send(handle, line)) {
      fprintf(stderr, "[new] %s\n", line);
    } else {
      fprintf(stderr, "[ignored] %s\n", line);
    }
  }

  mpclipboard_handle_stop(handle);

  return 0;
}

void *start_polling(void *data) {
  mpclipboard_handle_t *handle = data;
  while (true) {
    mpclipboard_output_t output = mpclipboard_handle_poll(handle);
    if (output.text) {
      printf("text = %s\n", output.text);
      free(output.text);
    }
    if (output.connectivity) {
      printf("connectivity = %s\n", *output.connectivity ? "true" : "false");
      free(output.connectivity);
    }

    usleep(100);
  }
  return NULL;
}
