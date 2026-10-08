#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
static int path_for(int fd,char *path,size_t n) {
 char link[64];snprintf(link,sizeof(link),"/proc/self/fd/%d",fd);ssize_t got=readlink(link,path,n-1);if(got<0)return 0;path[got]=0;return 1;
}
ssize_t write(int fd,const void*data,size_t n) {
 static ssize_t(*real)(int,const void*,size_t);if(!real)real=dlsym(RTLD_NEXT,"write");
 const char *mode=getenv("ALT_EXEC_FAULT");char path[4096];
 if(mode&&path_for(fd,path,sizeof(path))&&strstr(path,"/receipt.reserve")&&n) {
  int final=((const char*)data)[0]=='{';
  if((!strcmp(mode,"reserve-full")&&!final)||(!strcmp(mode,"finish-full")&&final)){errno=ENOSPC;return -1;}
  if(!strcmp(mode,"crash-before-commit")&&final)_exit(86);
 }
 return real(fd,data,n);
}
int fsync(int fd) {
 static int(*real)(int);if(!real)real=dlsym(RTLD_NEXT,"fsync");int value=real(fd);
 const char *mode=getenv("ALT_EXEC_FAULT");char path[4096],receipt[8192];
 if(mode&&!strcmp(mode,"crash-after-reserve")&&path_for(fd,path,sizeof(path))&&strstr(path,"/receipt.reserve")) { char first=0; if(pread(fd,&first,1,0)==1&&first=='{')_exit(88); }
 if(mode&&!strcmp(mode,"crash-after-commit")&&path_for(fd,path,sizeof(path))&&strstr(path,"/execution/")) {
  snprintf(receipt,sizeof(receipt),"%s/receipt.json",path);
  if(access(receipt,F_OK)==0)_exit(87);
 }
 return value;
}
