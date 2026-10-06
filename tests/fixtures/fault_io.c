#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
static int target(int fd, int temporary) {
 const char *root=getenv("ALT_FAULT_PROJECT");if(!root)return 0;
 char link[64],path[4096];snprintf(link,sizeof(link),"/proc/self/fd/%d",fd);
 ssize_t n=readlink(link,path,sizeof(path)-1);if(n<0)return 0;path[n]=0;
 const char *pattern=getenv("ALT_FAULT_MATCH");if(!pattern)pattern="/.alt-write-";
 return temporary?strncmp(path,root,strlen(root))==0&&(strcmp(pattern,"*")==0||strstr(path,pattern)!=NULL):strcmp(path,root)==0;
}
ssize_t write(int fd,const void *data,size_t n) {
 static ssize_t(*real)(int,const void*,size_t);if(!real)real=dlsym(RTLD_NEXT,"write");
 const char *mode=getenv("ALT_FAULT_MODE");
 if(mode&&target(fd,1)) {
  if(strcmp(mode,"enospc")==0){errno=ENOSPC;return -1;}
  if(strcmp(mode,"crash-before-rename")==0){ssize_t v=real(fd,data,n);(void)v;_exit(86);}
 }
 return real(fd,data,n);
}
int fsync(int fd) {
 static int(*real)(int);if(!real)real=dlsym(RTLD_NEXT,"fsync");
 int result=real(fd);const char *mode=getenv("ALT_FAULT_MODE");
 if(mode&&strcmp(mode,"crash-after-rename")==0&&target(fd,0))_exit(87);
 return result;
}
