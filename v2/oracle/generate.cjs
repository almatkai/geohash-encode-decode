// npm ci --ignore-scripts --prefix v2/oracle; node v2/oracle/generate.cjs
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const ts = require('typescript');
const base = path.resolve(__dirname, '../tests/fixtures/upstream/src');
const cache = new Map();
const react = { useMemo: f => f(), createElement: (type, props, ...children) => {
  props = { ...props, children: children.flat(Infinity) };
  return typeof type === 'function' ? type(props) : { type, props, children: props.children };
}};
const list = Object.assign('List', { Section: 'List.Section', Item: 'List.Item' });
const api = { List: list, Icon: { ExclamationMark: 'ExclamationMark' }, ActionPanel: 'ActionPanel',
  CopyToClipboardAction: 'Copy', OpenInBrowserAction: 'Open', getPreferenceValues: () => ({ closeOnAction: true }),
  closeMainWindow: () => { throw Error('No OS effects in oracle'); }, popToRoot: () => { throw Error('No OS effects in oracle'); } };
function load(file) {
  if (cache.has(file)) return cache.get(file).exports;
  const module = { exports: {} }; cache.set(file, module);
  const js = ts.transpileModule(fs.readFileSync(file, 'utf8'), { compilerOptions: { jsx: ts.JsxEmit.React, module: ts.ModuleKind.CommonJS, esModuleInterop: true, target: ts.ScriptTarget.ES2022 } }).outputText;
  function req(name) {
    if (name === '@raycast/api') return api;
    if (name === 'react') return react;
    if (name.startsWith('.')) {
      const resolved = path.resolve(path.dirname(file), name);
      return load(['.ts', '.tsx'].map(ext => resolved + ext).find(p => fs.existsSync(p)));
    }
    return require(name);
  }
  vm.runInNewContext(`(function(require,module,exports,React){${js}\n})`, { console })(req,module,module.exports,react);
  return module.exports;
}
const encoded = load(path.join(base, 'components/EncodedList.tsx')).default;
const decoded = load(path.join(base, 'components/DecodedInfo.tsx')).default;
function rows(tree, encode) {
  const result = []; let invalid;
  function walk(node, section = '') {
    if (!node || typeof node !== 'object') return;
    if (node.type === 'List.Section') section = node.props.title;
    if (node.type === 'List.Item') {
      if (node.props.icon === 'ExclamationMark') { invalid = node.props.title; return; }
      let actions = node.props.actions;
      const values = actions?.children || [];
      const copy = values.find(n => n?.type === 'Copy');
      const open = values.find(n => n?.type === 'Open');
      result.push({ section, title: node.props.title, value: encode ? node.props.accessoryTitle : copy.props.content, link: open?.props.url ?? null });
    }
    for (const c of node.children || []) walk(c, section);
  }
  walk(tree);
  return invalid ? { error: invalid } : { rows: result };
}
const coords = [
 ['40.4169','-3.7035'],['43.238949','76.889709'],['0','0'],['-0','+0'],['90','180'],['-90','-180'],
 ['180','180'],['-180','-180'],['90.00001','-3'],['123.456','45'],['-179.9','0'],['45','90'],['-45','-90'],
 ['22.5','45'],['1e-7','-1e-7'],['5e-324','-5e-324'],['-1e-9999','1e-9999'],['.5','5.'],['+1.0','01'],
 ['0x2d','0XB4'],['0b101101','0o55'],['\uFEFF\u00a040\u2029','\t-3\r\n'],['\u200040\u3000','\u202f-3\u205f'],
 ['','0'],[' ','0'],['\uFEFF','0'],['\u008540','0'],['\u200b40','0'],['181','0'],['-180.00001','0'],['NaN','0'],
 ['Infinity','0'],['-Infinity','0'],['1e309','0'],['inf','0'],['0x','0'],['+0x10','0'],['-0b1','0'],['0b2','0'],
 ['1_0','0'],['1e','0'],['.','0'],['1 2','0'],['１２','0'],['1f','0'],['0','181'],['0','NaN'],['0',''],
 ['0x'+'0'.repeat(100)+'b4','0'],['0x10000000000000000','0'],['180.00000000000001','-180.00000000000001']
];
let seed=0x12345678;
const random = () => { seed=(Math.imul(seed,1664525)+1013904223)>>>0; return seed/2**32; };
for(let i=0;i<180;i++) coords.push([(random()*360-180).toString(),(random()*360-180).toString()]);
const hashes = ['ezmju','u4pruydqqvj','0','z','b','p','s','7','zzzzzzzzzzzz','000000000000','bpbpbpbpbpbp','pbpbpbpbpbpb','s00000000000','7zzzzzzzzzzz',
 '', 'EZMJU','ezmjU',' ezmju','ezmju\n','a','i','l','o','é','😀','s/../z','https://evil.test','null',
 's'.repeat(22), 'z'.repeat(32), '0'.repeat(128), 'u4pruydqqvj'.repeat(94), '0'.repeat(1024), 'z'.repeat(1024)];
const alphabet='0123456789bcdefghjkmnpqrstuvwxyz';
for(let i=0;i<260;i++) hashes.push(Array.from({length:1+(i%65)},()=>alphabet[Math.floor(random()*32)]).join(''));
const fixture = { provenance: { upstream: 'raycast/extensions@6734d85992290066fd31a7f5bee3cca3d3545a1a/extensions/geohash-encode-decode', method: 'Execute pinned source TSX components using typescript transpileModule and pure React/Raycast UI mocks; real locked JS dependencies. Capture rendered rows/actions, never OS effects.', versions: Object.fromEntries(['ngeohash','@sindresorhus/is','geohash-to-geojson','wkt-parser-helper','wellknown'].map(n=>[n,require(n+'/package.json').version])) },
 encode: coords.map(([latitude,longitude])=>({latitude,longitude,...rows(encoded({latitude,longitude}),true)})),
 decode: hashes.map(geohash=>({geohash,...rows(decoded({geohash}),false)})) };
const out=path.resolve(__dirname,'../tests/fixtures/oracle.json');
fs.writeFileSync(out, JSON.stringify(fixture,null,2)+'\n');
console.log(`Wrote ${fixture.encode.length} encode / ${fixture.decode.length} decode vectors to ${out}`);
