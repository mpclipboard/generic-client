#include "mpclipboard-generic-client.h"
#include <stdio.h>
#include <stdlib.h>
#include <pthread.h>
#include <unistd.h>
#include <string.h>

void *poll(void* data);

int main() {
    mpclipboard_setup();

    mpclipboard_config_t *config = mpclipboard_config_read_from_xdg_config_dir();
    if (!config) {
        fprintf(stderr, "config is NULL\n");
        return 1;
    }

    mpclipboard_start_thread(config);

    pthread_t thread;
    pthread_create(&thread, NULL, poll, NULL);

    size_t line_length = 100;
    char *line = malloc(line_length);
    while(true) {
        getline(&line, &line_length, stdin);
        if (strcmp(line, "exit\n") == 0) {
            break;
        }
        mpclipboard_send(line);
    }

    mpclipboard_stop_thread();

    return 0;
}

void *poll(void* data) {
    while (true) {
        mpclipboard_output_t output = mpclipboard_poll();
        if (output.text) {
            printf("text = %s\n", output.text);
            free(output.text);
        }
        if (output.connectivity) {
            printf("connectivity = %s\n", output.connectivity ? "true" : "false");
            free(output.connectivity);
        }

        usleep(100);
    }
    return NULL;
}
