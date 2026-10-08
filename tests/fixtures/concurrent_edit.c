#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>
static int injected;
int renameat2(int oldfd,const char *oldname,int newfd,const char *newname,unsigned int flags) {
 static int(*real)(int,const char*,int,const char*,unsigned int);
 if(!real)real=dlsym(RTLD_NEXT,"renameat2");
 const char *mode=getenv("ALT_EXTERNAL_EDIT_MODE");
 const char *target=getenv("ALT_EXTERNAL_EDIT_TARGET");
 if(mode&&target&&!injected&&(strcmp(newname,target)==0||strcmp(oldname,target)==0)) {
  injected=1;
  if(strcmp(mode,"chmod")==0) {
   if(fchmodat(newfd,target,0600,0))_exit(93);
  } else {
  const char *name=strcmp(mode,"rename")==0?".external-save":target;
  int fd=openat(newfd,name,O_CREAT|O_TRUNC|O_WRONLY,0600);
  if(fd<0)_exit(90);
  const char body[]="USER_EDIT_AT_EXCHANGE\n";
  if(write(fd,body,sizeof(body)-1)!=(ssize_t)(sizeof(body)-1))_exit(91);
  fsync(fd);close(fd);
  if(strcmp(mode,"rename")==0&&renameat(newfd,name,newfd,target))_exit(92);
  }
 }
 int result=real(oldfd,oldname,newfd,newname,flags);
 if(injected&&getenv("ALT_EXTERNAL_AFTER_RENAME_CRASH")&&result==0)_exit(87);
 return result;
}
int fsync(int descriptor) {
 static int(*real)(int);
 static int directories_synced;
 if(!real)real=dlsym(RTLD_NEXT,"fsync");
 int result=real(descriptor);
 const char *root=getenv("ALT_EXTERNAL_CWD");
 struct stat metadata;
 char link[64],path[4096];
 snprintf(link,sizeof(link),"/proc/self/fd/%d",descriptor);
 ssize_t length=readlink(link,path,sizeof(path)-1);
 if(length>=0)path[length]=0;
 if(getenv("ALT_FINALIZE_CRASH")&&root&&length>=0&&strcmp(path,root)==0&&fstat(descriptor,&metadata)==0&&S_ISDIR(metadata.st_mode)&&++directories_synced==2) {
  int fd=openat(descriptor,".external-final-save",O_CREAT|O_TRUNC|O_WRONLY,0644);
  if(fd<0)_exit(94);
  const char body[]="after\n";
  if(write(fd,body,sizeof(body)-1)!=(ssize_t)(sizeof(body)-1))_exit(95);
  real(fd);close(fd);
  if(renameat(descriptor,".external-final-save",descriptor,"answer.py"))_exit(96);
  _exit(88);
 }
 return result;
}
