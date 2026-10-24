// Runs actual functions extracted by AST from byte-exact beta renderer files.
// This is optional Node QA; cargo's adapter tests are self-contained.
const fs=require('node:fs');
const path=require('node:path');
const cp=require('node:child_process');
const assert=require('node:assert/strict');
const ts=require('typescript');
const root=path.resolve(__dirname,'..');
const fixture=path.join(root,'target/renderer-wire.json');
const run=cp.spawnSync('cargo',['test','--locked','--manifest-path',path.join(root,'Cargo.toml'),'--test','contract','renderer_wire_contract_samples'],{stdio:'inherit',env:{...process.env,TEZBAR_GEOHASH_RENDERER_FIXTURE:fixture}});
if(run.status!==0) process.exit(run.status||1);
function functions(file,names) {
 const source=ts.createSourceFile(file,fs.readFileSync(path.join(root,'tests/fixtures/beta',file),'utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
 const nodes=source.statements.filter(n=>ts.isFunctionDeclaration(n)&&names.includes(n.name?.text));
 assert.equal(nodes.length,names.length);
 const code=nodes.map(n=>n.getText(source).replace(/^export /,'')).join('\n');
 const js=ts.transpileModule(code,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText;
 return new Function(js+`\nreturn {${names.join(',')}};`)();
}
const list=functions('list-runtime.tsx',['cleanSubtitle','parseRowIcon','textValue','parseAccessories','parseDetailFromChildren','parseListRows']);
const form=functions('form-runtime.tsx',['collectOptionData','collectFormItems','sameFormValue','reconcileFormValues']);
const nav=functions('index.tsx',['backActionId']);
const oracle=JSON.parse(fs.readFileSync(path.join(root,'tests/fixtures/oracle.json'),'utf8'));
for(const sample of JSON.parse(fs.readFileSync(fixture,'utf8'))) {
 const encode=sample.command==='encode-coordinates';
 const fields=form.collectFormItems(sample.form.root).filter(n=>n.type==='field').map(n=>n.field);
 assert.deepEqual(form.reconcileFormValues(fields),encode?{latitude:'',longitude:''}:{geohash:''});
 const changed=form.collectFormItems(sample.changed.root).filter(n=>n.type==='field').map(n=>n.field);
 assert.equal(form.reconcileFormValues(changed)[encode?'latitude':'geohash'],encode?'40.4169':'ezmju');
 const back=form.collectFormItems(sample.back.root).filter(n=>n.type==='field').map(n=>n.field);
 assert.deepEqual(form.reconcileFormValues(back),encode?{latitude:'40.4169',longitude:'-3.7035'}:{geohash:'ezmju'});
 for(const field of fields) assert(sample.form.actions.some(a=>a.id===field.actionId));
 assert(sample.form.actions.some(a=>a.kind==='submit-form'));
 const actual=list.parseListRows(sample.results.root);
 const expected=(encode?oracle.encode.find(v=>v.latitude==='40.4169'&&v.longitude==='-3.7035'):oracle.decode.find(v=>v.geohash==='ezmju')).rows;
 assert.deepEqual(actual.map(r=>({section:r.section||'',title:r.title,value:r.accessories[0].text})),expected.map(({section,title,value})=>({section,title,value})));
 for(const row of actual) for(const id of row.actionIds) assert(sample.results.actions.some(a=>a.id===id));
 assert(sample.results.actions.some(a=>a.id===nav.backActionId(sample.results.root)&&a.title==='Back'));
}
console.log('Actual beta form/list/back parsers accept adapter views and preserve every displayed source value.');
