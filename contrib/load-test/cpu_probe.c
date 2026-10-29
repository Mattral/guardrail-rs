// What N busy threads get out of the machine, to read the ONNX scaling
// numbers against:
//
//   compute    each thread runs AVX2 fused multiply-adds on registers only
//              (no memory traffic); prints GFLOP/s per thread, which falls
//              when more busy cores lower the clock.
//   bandwidth  each thread sums its own 256 MiB buffer with AVX2 loads,
//              over and over; prints the total read rate in GB/s.
//
//   cc -O2 -mavx2 -mfma -pthread -o cpu_probe contrib/load-test/cpu_probe.c
//   for n in 1 2 4 8 16; do ./cpu_probe bandwidth $n 6; done
//   ./cpu_probe compute 4 75 &   # interference load next to a benchmark

#define _GNU_SOURCE
#include <immintrin.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static int bandwidth_mode;
static volatile int stop;
static volatile float keep; // stops the compiler dropping the loops

typedef struct {
    double work; // flops or bytes
    float sink;  // keeps the result live
} result;

static double now(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return t.tv_sec + t.tv_nsec * 1e-9;
}

static void *compute(result *r) {
    __m256 a[10];
    __m256 b = _mm256_set1_ps(0.999f), c = _mm256_set1_ps(0.001f);
    for (int i = 0; i < 10; i++) a[i] = _mm256_set1_ps((float)i);
    long iters = 0;
    while (!stop) {
        for (int k = 0; k < 1000; k++)
            for (int i = 0; i < 10; i++) a[i] = _mm256_fmadd_ps(a[i], b, c);
        iters += 1000;
    }
    __m256 s = a[0];
    for (int i = 1; i < 10; i++) s = _mm256_add_ps(s, a[i]);
    r->sink = _mm256_cvtss_f32(s);
    r->work = (double)iters * 10 * 8 * 2; // 10 vectors x 8 lanes x (mul + add)
    return NULL;
}

static void *bandwidth(result *r) {
    size_t n = (256u << 20) / sizeof(__m256);
    __m256 *buf = aligned_alloc(32, n * sizeof(__m256));
    memset(buf, 1, n * sizeof(__m256));
    __m256 s0 = _mm256_setzero_ps(), s1 = s0, s2 = s0, s3 = s0;
    double bytes = 0;
    while (!stop) {
        for (size_t i = 0; i < n; i += 4) {
            s0 = _mm256_add_ps(s0, buf[i]);
            s1 = _mm256_add_ps(s1, buf[i + 1]);
            s2 = _mm256_add_ps(s2, buf[i + 2]);
            s3 = _mm256_add_ps(s3, buf[i + 3]);
        }
        bytes += (double)n * sizeof(__m256);
    }
    r->sink = _mm256_cvtss_f32(_mm256_add_ps(_mm256_add_ps(s0, s1), _mm256_add_ps(s2, s3)));
    r->work = bytes;
    free(buf);
    return NULL;
}

static void *worker(void *arg) {
    return bandwidth_mode ? bandwidth(arg) : compute(arg);
}

int main(int argc, char **argv) {
    if (argc != 4 || (strcmp(argv[1], "compute") && strcmp(argv[1], "bandwidth"))) {
        fprintf(stderr, "usage: %s compute|bandwidth THREADS SECONDS\n", argv[0]);
        return 2;
    }
    bandwidth_mode = strcmp(argv[1], "bandwidth") == 0;
    int n = atoi(argv[2]);
    double secs = atof(argv[3]);
    if (n < 1 || n > 256) return 2;
    pthread_t *t = calloc(n, sizeof *t);
    result *r = calloc(n, sizeof *r);

    double start = now();
    for (int i = 0; i < n; i++) pthread_create(&t[i], NULL, worker, &r[i]);
    struct timespec d = {(time_t)secs, (long)((secs - (time_t)secs) * 1e9)};
    nanosleep(&d, NULL);
    stop = 1;
    double total = 0;
    float sink = 0;
    for (int i = 0; i < n; i++) {
        pthread_join(t[i], NULL);
        total += r[i].work;
        sink += r[i].sink;
    }
    double elapsed = now() - start;

    if (bandwidth_mode)
        printf("bandwidth threads=%d total=%.1f GB/s per_thread=%.1f GB/s\n", n,
               total / elapsed / 1e9, total / n / elapsed / 1e9);
    else
        printf("compute threads=%d per_thread=%.1f GFLOP/s total=%.1f GFLOP/s\n", n,
               total / n / elapsed / 1e9, total / elapsed / 1e9);
    keep = sink;
    return 0;
}
