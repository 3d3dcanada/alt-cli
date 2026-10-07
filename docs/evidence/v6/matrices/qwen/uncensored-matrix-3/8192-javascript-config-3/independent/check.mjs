const {label}=await import(process.env.ALT_PROJECT_URL+'/labels.js');
if(label(' Cedar ')!=='CEDAR'||label('v2')!=='V2')throw Error('incorrect label');
const fs=await import('node:fs');const cp=await import('node:child_process');
if(JSON.parse(fs.readFileSync('package.json','utf8')).type!=='module')throw Error('module type changed');
if(cp.execFileSync(process.execPath,['main.js'],{encoding:'utf8'}).trim()!=='CEDAR')throw Error('entry point failed');

console.log("ALT_ORACLE_COMPLETED_9cb17a9750334769be8e64e67b1453e2");
