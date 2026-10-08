use alt_cli::project::{Project,Policy};
use std::{fs,path::PathBuf,os::unix::fs::PermissionsExt};
fn main(){
 let args=std::env::args().collect::<Vec<_>>(); let base=PathBuf::from(&args[1]); let mode=&args[2];
 let cwd=base.join("project");let data=base.join("state"); fs::create_dir_all(&cwd).unwrap();
 let file=cwd.join("answer.py");fs::write(&file,"before\n").unwrap();fs::set_permissions(&file,fs::Permissions::from_mode(0o644)).unwrap();
 let p=Project::open(&data,&cwd).unwrap();p.start_task("t","audit external edit preservation").unwrap();p.note("t","plan","Read edit verify","user").unwrap();p.read("t","answer.py",1,10).unwrap();
 let c=p.prepare_edit("t","answer.py",None,"before","after","replace","Audit preservation").unwrap();
 if mode=="permissions"{fs::set_permissions(&file,fs::Permissions::from_mode(0o600)).unwrap();}
 let applied=p.apply(&c.id,Policy::Trusted);println!("apply_success={}",applied.is_ok());println!("after_apply={:?}",fs::read_to_string(&file).unwrap());println!("after_apply_mode={:o}",fs::metadata(&file).unwrap().permissions().mode()&0o777);
 if mode=="permissions"{fs::set_permissions(&file,fs::Permissions::from_mode(0o640)).unwrap();}
 println!("undo_success={}",p.undo(&c.id).is_ok());println!("after_undo={:?}",fs::read_to_string(&file).unwrap());println!("after_undo_mode={:o}",fs::metadata(&file).unwrap().permissions().mode()&0o777);
}
