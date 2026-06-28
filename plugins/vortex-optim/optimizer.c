#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <dirent.h>
#include <fcntl.h>
#include <sys/uio.h>

static int find_vortex_pid(void) {
    int best_pid = -1;
    long best_rss = 0;
    DIR *proc = opendir("/proc");
    if (!proc) return -1;
    struct dirent *entry;
    while ((entry = readdir(proc))) {
        if (entry->d_name[0] < '0' || entry->d_name[0] > '9') continue;
        int pid = atoi(entry->d_name);
        if (pid <= 0) continue;
        char path[4096];
        snprintf(path, sizeof(path), "/proc/%d/cmdline", pid);
        int fd = open(path, O_RDONLY);
        if (fd < 0) continue;
        char cmd[4096];
        int n = read(fd, cmd, sizeof(cmd) - 1);
        close(fd);
        if (n <= 0) continue;
        cmd[n] = 0;
        if (!strstr(cmd, "Vortex.exe")) continue;
        snprintf(path, sizeof(path), "/proc/%d/status", pid);
        FILE *sf = fopen(path, "r");
        if (!sf) continue;
        long rss = 0;
        char line[256];
        while (fgets(line, sizeof(line), sf)) {
            if (sscanf(line, "VmRSS: %ld kB", &rss) == 1) break;
        }
        fclose(sf);
        if (rss > best_rss) {
            best_rss = rss;
            best_pid = pid;
        }
    }
    closedir(proc);
    return best_pid;
}

#define SPEED_CAP_ADDR 0x143910aa4UL

int main(void) {
    fprintf(stderr, "[vortex-optim] waiting for Vortex.exe...\n");
    int pid = -1;
    for (int i = 0; i < 60; i++) {
        pid = find_vortex_pid();
        if (pid > 0) break;
        sleep(1);
    }
    if (pid <= 0) {
        fprintf(stderr, "[vortex-optim] could not find Vortex.exe\n");
        return 1;
    }
    fprintf(stderr, "[vortex-optim] found PID %d\n", pid);
    sleep(2);

    fprintf(stderr, "[vortex-optim] patching speed cap... ");
    float val = 999.0f;
    struct iovec liov = { &val, sizeof(val) };
    struct iovec riov = { (void *)SPEED_CAP_ADDR, sizeof(val) };
    ssize_t n = process_vm_writev(pid, &liov, 1, &riov, 1, 0);
    if (n == sizeof(val))
        fprintf(stderr, "ok\n");
    else
        fprintf(stderr, "skipped (not supported)\n");

    fprintf(stderr, "[vortex-optim] done\n");
    return 0;
}
