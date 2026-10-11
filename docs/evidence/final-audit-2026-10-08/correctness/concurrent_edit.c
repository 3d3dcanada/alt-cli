#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
static int injected=0;
int fsync(int fd){
 static int(*real)(int); if(!real)real=dlsym(RTLD_NEXT,"fsync");int result=real(fd);
 const char *victim=getenv("ALT_AUDIT_TARGET"); if(victim&&!injected){
 char link[64],path[4096];snprintf(link,sizeof(link),"/proc/self/fd/%d",fd);ssize_t n=readlink(link,path,sizeof(path)-1);if(n>0){path[n]=0;if(strstr(path,"/.alt-write-")!=NULL){injected=1;int out=open(victim,O_WRONLY|O_TRUNC);if(out>=0){const char*value="USER_EDIT_AFTER_STALE_CHECK\n";write(out,value,strlen(value));real(out);close(out);fprintf(stderr,"injected_editor_write_before_rename=1\n");}}}
 }
 return result;
}
