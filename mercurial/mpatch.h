#ifndef _HG_MPATCH_H_
#define _HG_MPATCH_H_

struct mpatch_frag {
	int start, end, len;
	const char *data;
};

struct mpatch_flist {
	struct mpatch_frag *base, *head, *tail;
};

int mpatch_decode(const char *bin, ssize_t len, struct mpatch_flist** res);
ssize_t mpatch_calcsize(ssize_t len, struct mpatch_flist *l);
void mpatch_lfree(struct mpatch_flist *a);
int mpatch_apply(char *buf, const char *orig, ssize_t len,
	struct mpatch_flist *l);
struct mpatch_flist *mpatch_fold(void *bins, ssize_t start, ssize_t end);

#endif
