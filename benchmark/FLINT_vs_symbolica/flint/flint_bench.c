/* FLINT side of benchmark/FLINT_vs_symbolica (benchmark tooling, not production).
 * Reads a plain integer term-list operand file ("symgcad-flint-bench-op 1"),
 * runs one exact operation single threaded with one FLINT kernel, writes
 * timing JSON and dumps the exact output as "coefficient e0 e1 ..." lines
 * (hashed by scripts/canon.py).
 *
 * usage: flint_bench <op-file> <kernel: mpoly|poly|polyeuclid> <json-out> <dump-out|-> [min_seconds] [max_reps]
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <flint/flint.h>
#include <flint/fmpz.h>
#include <flint/fmpz_poly.h>
#include <flint/fmpz_poly_factor.h>
#include <flint/fmpz_mpoly.h>
#include <flint/fmpz_mpoly_factor.h>

#define MAXV 64

static char operation[64];
static char names[MAXV][64];
static slong nvars, var, npolys;

static double now(void)
{
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return t.tv_sec + 1e-9 * t.tv_nsec;
}

static char *read_line(FILE *f, char **buf, size_t *cap)
{
    ssize_t n = getline(buf, cap, f);
    if (n < 0) { fprintf(stderr, "truncated operand file\n"); exit(2); }
    if (n > 0 && (*buf)[n - 1] == '\n') (*buf)[n - 1] = 0;
    return *buf;
}

static void read_op(const char *path, fmpz_mpoly_struct **polys, fmpz_mpoly_ctx_t ctx)
{
    FILE *f = fopen(path, "r");
    char *buf = NULL; size_t cap = 0;
    if (!f) { perror(path); exit(2); }
    if (strcmp(read_line(f, &buf, &cap), "symgcad-flint-bench-op 1")) { fprintf(stderr, "magic\n"); exit(2); }
    if (sscanf(read_line(f, &buf, &cap), "operation %63s", operation) != 1) exit(2);
    read_line(f, &buf, &cap);
    if (strncmp(buf, "variables ", 10)) exit(2);
    nvars = 0;
    for (char *tok = strtok(buf + 10, " "); tok; tok = strtok(NULL, " ")) {
        if (nvars >= MAXV) exit(2);
        strncpy(names[nvars++], tok, 63);
    }
    if (sscanf(read_line(f, &buf, &cap), "variable %ld", &var) != 1) exit(2);
    if (sscanf(read_line(f, &buf, &cap), "polynomials %ld", &npolys) != 1) exit(2);
    fmpz_mpoly_ctx_init(ctx, nvars, ORD_LEX);
    *polys = flint_malloc(npolys * sizeof(fmpz_mpoly_struct));
    ulong exp[MAXV];
    fmpz_t c; fmpz_init(c);
    for (slong i = 0; i < npolys; i++) {
        slong terms;
        fmpz_mpoly_init(*polys + i, ctx);
        if (sscanf(read_line(f, &buf, &cap), "terms %ld", &terms) != 1) exit(2);
        for (slong t = 0; t < terms; t++) {
            read_line(f, &buf, &cap);
            char *sp = strchr(buf, ' ');
            if (sp) *sp = 0;
            if (fmpz_set_str(c, buf, 10)) { fprintf(stderr, "coefficient\n"); exit(2); }
            char *p = sp ? sp + 1 : NULL;
            for (slong v = 0; v < nvars; v++) {
                if (!p) { fprintf(stderr, "exponents\n"); exit(2); }
                exp[v] = strtoul(p, &p, 10);
                while (p && *p == ' ') p++;
            }
            fmpz_mpoly_push_term_fmpz_ui(*polys + i, c, exp, ctx);
        }
        fmpz_mpoly_sort_terms(*polys + i, ctx);
        fmpz_mpoly_combine_like_terms(*polys + i, ctx);
    }
    fmpz_clear(c);
    free(buf);
    fclose(f);
}

/* variable index in which p is univariate (-1 if constant, -2 if several) */
static slong univariate_variable(const fmpz_mpoly_t p, const fmpz_mpoly_ctx_t ctx)
{
    slong found = -1;
    slong deg[MAXV];
    fmpz_mpoly_degrees_si(deg, p, ctx);
    for (slong v = 0; v < nvars; v++)
        if (deg[v] > 0) { if (found >= 0) return -2; found = v; }
    return found;
}

static void dump_mpoly(FILE *out, const fmpz_mpoly_t p, ulong mult, const fmpz_mpoly_ctx_t ctx,
                       slong *terms, slong *bits)
{
    ulong exp[MAXV];
    fmpz_t c; fmpz_init(c);
    fprintf(out, "poly %lu\n", mult);
    slong len = fmpz_mpoly_length(p, ctx);
    *terms += len;
    for (slong i = 0; i < len; i++) {
        fmpz_mpoly_get_term_coeff_fmpz(c, p, i, ctx);
        fmpz_mpoly_get_term_exp_ui(exp, p, i, ctx);
        slong b = fmpz_bits(c);
        if (b > *bits) *bits = b;
        fmpz_fprint(out, c);
        for (slong v = 0; v < nvars; v++) fprintf(out, " %lu", exp[v]);
        fputc('\n', out);
    }
    fmpz_clear(c);
}

static void dump_fmpz_poly(FILE *out, const fmpz_poly_t p, ulong mult, slong v, slong *terms, slong *bits)
{
    fprintf(out, "poly %lu\n", mult);
    for (slong k = 0; k <= fmpz_poly_degree(p); k++) {
        const fmpz *c = fmpz_poly_get_coeff_ptr(p, k);
        if (fmpz_is_zero(c)) continue;
        (*terms)++;
        if ((slong) fmpz_bits(c) > *bits) *bits = fmpz_bits(c);
        fmpz_fprint(out, c);
        for (slong w = 0; w < nvars; w++) fprintf(out, " %ld", w == v ? k : 0);
        fputc('\n', out);
    }
}

static void dump_constant(FILE *out, const fmpz_t c, slong *terms, slong *bits)
{
    fprintf(out, "poly 1\n");
    if (fmpz_is_zero(c)) return;
    (*terms)++;
    *bits = fmpz_bits(c);
    fmpz_fprint(out, c);
    for (slong w = 0; w < nvars; w++) fprintf(out, " 0");
    fputc('\n', out);
}

int main(int argc, char **argv)
{
    if (argc < 5) { fprintf(stderr, "usage\n"); return 2; }
    const char *op_path = argv[1], *kernel = argv[2], *json_path = argv[3], *dump_path = argv[4];
    double min_seconds = argc > 5 ? atof(argv[5]) : 1.0;
    long max_reps = argc > 6 ? atol(argv[6]) : 25;
    flint_set_num_threads(1);

    fmpz_mpoly_ctx_t ctx;
    fmpz_mpoly_struct *polys;
    double t0 = now();
    read_op(op_path, &polys, ctx);
    double parse_seconds = now() - t0;

    int is_euclid = !strcmp(kernel, "polyeuclid");
    int is_poly = !strcmp(kernel, "poly") || is_euclid;
    int is_res = !strcmp(operation, "resultant"), is_disc = !strcmp(operation, "discriminant");
    int is_factor = !strcmp(operation, "factor"), is_gcd = !strcmp(operation, "gcd");
    slong uv = -1; /* univariate variable for the poly kernel */
    fmpz_poly_struct up[2];
    if (is_poly) {
        for (slong i = 0; i < npolys; i++) {
            slong u = univariate_variable(polys + i, ctx);
            if (u == -2 || (u >= 0 && uv >= 0 && u != uv)) { fprintf(stderr, "not univariate\n"); return 3; }
            if (u >= 0) uv = u;
        }
        if ((is_res || is_disc) && uv >= 0 && uv != var) { fprintf(stderr, "not univariate in var\n"); return 3; }
        if (uv < 0) uv = (is_res || is_disc) ? var : 0;
        for (slong i = 0; i < npolys; i++) {
            fmpz_poly_init(up + i);
            if (!fmpz_mpoly_get_fmpz_poly(up + i, polys + i, uv, ctx)) { fprintf(stderr, "conversion\n"); return 3; }
        }
    } else if (strcmp(kernel, "mpoly")) { fprintf(stderr, "kernel\n"); return 2; }

    double times[1000];
    long reps = 0;
    double total = 0;
    int success = 1;
    fmpz_mpoly_t R; fmpz_mpoly_init(R, ctx);
    fmpz_mpoly_factor_t F; fmpz_mpoly_factor_init(F, ctx);
    fmpz_t r; fmpz_init(r);
    fmpz_poly_t g; fmpz_poly_init(g);
    fmpz_poly_factor_t pf; fmpz_poly_factor_init(pf);
    if (max_reps > 1000) max_reps = 1000;
    for (;;) {
        /* fresh outputs for every repetition */
        fmpz_mpoly_clear(R, ctx); fmpz_mpoly_init(R, ctx);
        fmpz_mpoly_factor_clear(F, ctx); fmpz_mpoly_factor_init(F, ctx);
        fmpz_clear(r); fmpz_init(r);
        fmpz_poly_clear(g); fmpz_poly_init(g);
        fmpz_poly_factor_clear(pf); fmpz_poly_factor_init(pf);
        double s = now();
        if (!is_poly) {
            if (is_res) success = fmpz_mpoly_resultant(R, polys + 0, polys + 1, var, ctx);
            else if (is_disc) success = fmpz_mpoly_discriminant(R, polys + 0, var, ctx);
            else if (is_factor) success = fmpz_mpoly_factor(F, polys + 0, ctx);
            else if (is_gcd) success = fmpz_mpoly_gcd(R, polys + 0, polys + 1, ctx);
        } else {
            if (is_res && is_euclid) fmpz_poly_resultant_euclidean(r, up + 0, up + 1);
            else if (is_res) fmpz_poly_resultant(r, up + 0, up + 1);
            else if (is_disc) fmpz_poly_discriminant(r, up + 0);
            else if (is_factor) fmpz_poly_factor(pf, up + 0);
            else if (is_gcd) fmpz_poly_gcd(g, up + 0, up + 1);
        }
        double t = now() - s;
        times[reps++] = t;
        total += t;
        if (total >= min_seconds || reps >= max_reps) break;
    }
    slong terms = 0, bits = 0;
    if (strcmp(dump_path, "-")) {
        FILE *out = fopen(dump_path, "w");
        if (!is_poly) {
            if (is_factor) for (slong i = 0; i < F->num; i++)
                dump_mpoly(out, F->poly + i, fmpz_get_ui(F->exp + i), ctx, &terms, &bits);
            else dump_mpoly(out, R, 1, ctx, &terms, &bits);
        } else {
            if (is_res || is_disc) dump_constant(out, r, &terms, &bits);
            else if (is_gcd) dump_fmpz_poly(out, g, 1, uv, &terms, &bits);
            else for (slong i = 0; i < pf->num; i++)
                dump_fmpz_poly(out, pf->p + i, pf->exp[i], uv, &terms, &bits);
        }
        fclose(out);
    }
    /* sort for min/median */
    double sorted[1000];
    memcpy(sorted, times, reps * sizeof(double));
    for (long i = 1; i < reps; i++) for (long j = i; j > 0 && sorted[j - 1] > sorted[j]; j--) {
        double x = sorted[j]; sorted[j] = sorted[j - 1]; sorted[j - 1] = x;
    }
    FILE *js = fopen(json_path, "w");
    fprintf(js, "{\"side\":\"flint\",\"backend\":\"flint-%s\",\"op\":\"%s\",\"operation\":\"%s\",\"kernel\":\"%s\","
                "\"chosen\":\"%s\",\"success\":%s,\"parse_seconds\":%.9f,\"reps\":%ld,\"min_seconds\":%.9f,"
                "\"median_seconds\":%.9f,\"output_terms\":%ld,\"output_max_bits\":%ld,\"times\":[",
            flint_version, op_path, operation, kernel,
            is_poly ? (is_res ? (is_euclid ? "fmpz_poly_resultant_euclidean" : "fmpz_poly_resultant") : is_disc ? "fmpz_poly_discriminant" : is_factor ? "fmpz_poly_factor" : "fmpz_poly_gcd")
                    : (is_res ? "fmpz_mpoly_resultant" : is_disc ? "fmpz_mpoly_discriminant" : is_factor ? "fmpz_mpoly_factor" : "fmpz_mpoly_gcd"),
            success ? "true" : "false", parse_seconds, reps, sorted[0], sorted[reps / 2], terms, bits);
    for (long i = 0; i < reps; i++) fprintf(js, "%s%.9f", i ? "," : "", times[i]);
    fprintf(js, "]}\n");
    fclose(js);
    return success ? 0 : 4;
}
