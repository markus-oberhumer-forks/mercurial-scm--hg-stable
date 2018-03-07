/*
 *  LibXDiff by Davide Libenzi ( File Differential Library )
 *  Copyright (C) 2003  Davide Libenzi
 *
 *  This library is free software; you can redistribute it and/or
 *  modify it under the terms of the GNU Lesser General Public
 *  License as published by the Free Software Foundation; either
 *  version 2.1 of the License, or (at your option) any later version.
 *
 *  This library is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 *  Lesser General Public License for more details.
 *
 *  You should have received a copy of the GNU Lesser General Public
 *  License along with this library; if not, see
 *  <http://www.gnu.org/licenses/>.
 *
 *  Davide Libenzi <davidel@xmailserver.org>
 *
 */

#if !defined(XDIFF_H)
#define XDIFF_H

#ifdef __cplusplus
extern "C" {
#endif /* #ifdef __cplusplus */

#include <stddef.h> /* size_t */

/* xpparm_t.flags */
#define XDF_NEED_MINIMAL (1 << 0)

#define XDF_INDENT_HEURISTIC (1 << 23)

/* emit bdiff-style "matched" (a1, a2, b1, b2) hunks instead of "different"
 * (a1, a2 - a1, b1, b2 - b1) hunks */
#define XDL_EMIT_BDIFFHUNK (1 << 4)

/* merge simplification levels */
#define XDL_MERGE_MINIMAL 0
#define XDL_MERGE_EAGER 1
#define XDL_MERGE_ZEALOUS 2
#define XDL_MERGE_ZEALOUS_ALNUM 3

/* merge favor modes */
#define XDL_MERGE_FAVOR_OURS 1
#define XDL_MERGE_FAVOR_THEIRS 2
#define XDL_MERGE_FAVOR_UNION 3

/* merge output styles */
#define XDL_MERGE_DIFF3 1

typedef struct s_mmfile {
	char *ptr;
	long size;
} mmfile_t;

typedef struct s_mmbuffer {
	char *ptr;
	long size;
} mmbuffer_t;

typedef struct s_xpparam {
	unsigned long flags;
} xpparam_t;

typedef struct s_xdemitcb {
	void *priv;
} xdemitcb_t;

typedef int (*xdl_emit_hunk_consume_func_t)(long start_a, long count_a,
					    long start_b, long count_b,
					    void *cb_data);

typedef struct s_xdemitconf {
	unsigned long flags;
	xdl_emit_hunk_consume_func_t hunk_func;
} xdemitconf_t;


#define xdl_malloc(x) malloc(x)
#define xdl_free(ptr) free(ptr)
#define xdl_realloc(ptr,x) realloc(ptr,x)

void *xdl_mmfile_first(mmfile_t *mmf, long *size);
long xdl_mmfile_size(mmfile_t *mmf);

int xdl_diff(mmfile_t *mf1, mmfile_t *mf2, xpparam_t const *xpp,
	     xdemitconf_t const *xecfg, xdemitcb_t *ecb);

typedef struct s_xmparam {
	xpparam_t xpp;
	int marker_size;
	int level;
	int favor;
	int style;
	const char *ancestor;	/* label for orig */
	const char *file1;	/* label for mf1 */
	const char *file2;	/* label for mf2 */
} xmparam_t;

#define DEFAULT_CONFLICT_MARKER_SIZE 7

int xdl_merge(mmfile_t *orig, mmfile_t *mf1, mmfile_t *mf2,
		xmparam_t const *xmp, mmbuffer_t *result);

#ifdef __cplusplus
}
#endif /* #ifdef __cplusplus */

#endif /* #if !defined(XDIFF_H) */
