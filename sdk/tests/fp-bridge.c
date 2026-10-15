/* SPDX-License-Identifier: Apache-2.0 */
struct mixed {double value; unsigned long count;};
extern double rust_double(double,double);
extern struct mixed rust_mixed(struct mixed);
extern unsigned long rust_stack(unsigned long,unsigned long,unsigned long,unsigned long,unsigned long,unsigned long,unsigned long);
extern double rust_fp_stack(double,double,double,double,double,double,double,double,double);
double c_double(double a,double b){return a+b;}
double c_to_rust(double a,double b){return rust_double(a,b);}
struct mixed c_mixed(struct mixed x){return rust_mixed(x);}
unsigned long c_stack(unsigned long a,unsigned long b,unsigned long c,unsigned long d,unsigned long e,unsigned long f,unsigned long g){return a+b+c+d+e+f+g;}
unsigned long c_to_rust_stack(unsigned long a,unsigned long b,unsigned long c,unsigned long d,unsigned long e,unsigned long f,unsigned long g){return rust_stack(a,b,c,d,e,f,g);}
double c_fp_stack(double a,double b,double c,double d,double e,double f,double g,double h,double i){return a+b+c+d+e+f+g+h+i;}
double c_to_rust_fp_stack(double a,double b,double c,double d,double e,double f,double g,double h,double i){return rust_fp_stack(a,b,c,d,e,f,g,h,i);}
