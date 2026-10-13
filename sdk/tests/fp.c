/* SPDX-License-Identifier: Apache-2.0 */
struct mixed {double value; unsigned long count;};
extern double rust_double(double,double);
extern struct mixed rust_mixed(struct mixed);
double c_double(double a,double b){return a+b;}
double c_to_rust(double a,double b){return rust_double(a,b);}
struct mixed c_mixed(struct mixed x){return rust_mixed(x);}
unsigned long c_stack(unsigned long a,unsigned long b,unsigned long c,unsigned long d,unsigned long e,unsigned long f,unsigned long g){return a+b+c+d+e+f+g;}
