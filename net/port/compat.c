// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#include <stddef.h>
void *memcpy(void *d, const void *s, size_t n) { unsigned char *a=d; const unsigned char *b=s; while(n--) *a++=*b++; return d; }
void *memmove(void *d, const void *s, size_t n) { unsigned char *a=d; const unsigned char *b=s; if ((size_t)a < (size_t)b) return memcpy(d,s,n); while(n) { --n; a[n]=b[n]; } return d; }
void *memset(void *d, int c, size_t n) { unsigned char *a=d; while(n--) *a++=(unsigned char)c; return d; }
int memcmp(const void *a, const void *b, size_t n) { const unsigned char *x=a,*y=b; while(n--) { if(*x!=*y) return *x-*y; x++; y++; } return 0; }
size_t strlen(const char *s) { size_t n=0; while(s[n]) n++; return n; }
int strncmp(const char *a,const char *b,size_t n) { while(n--) { unsigned char x=*a++,y=*b++; if(x!=y) return x-y; if(!x) return 0; } return 0; }
int atoi(const char *s) { unsigned int n=0; int sign=1; while(*s==' ' || (*s>='\t' && *s<='\r')) s++; if(*s=='-' || *s=='+') { if(*s=='-') sign=-1;s++; } while(*s>='0' && *s<='9') { unsigned int digit=(unsigned int)(*s++-'0'); if(n>214748364U || (n==214748364U && digit>7U)) return 0; n=n*10+digit; } return sign*(int)n; }
