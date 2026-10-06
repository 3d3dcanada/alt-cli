const {label}=await import(process.env.ALT_PROJECT_URL+'/labels.js');
if(label(' Cedar ')!=='CEDAR'||label('v2')!=='V2')throw Error('incorrect label');
