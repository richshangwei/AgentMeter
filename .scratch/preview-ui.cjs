const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname,'..');
const providers = ['codex','claude','copilot','antigravity'].map((provider,index)=>({
  provider,availability:index===2?'needs_login':'available',collection_state:index===2?'error':'ready',freshness:'fresh',collected_at:Date.now(),failure_code:index===2?'authentication_required':null,
  quota_windows:index===2?[]:[{label:index%2?'目前工作階段':'5 小時',remaining_percent:[82,47,0,8][index],reset_display:'2 小時 14 分'},{label:'每週',remaining_percent:[61,28,0,8][index]}]
}));
const tabletSnapshot={stream_id:'preview',revision:1,provider_data:'live',providers};
const desktopSnapshot={refreshing:false,provider_states:providers};
const types={'.html':'text/html; charset=utf-8','.js':'text/javascript; charset=utf-8','.css':'text/css; charset=utf-8','.png':'image/png'};

function sendFile(response,file){const extension=path.extname(file);response.writeHead(200,{'Content-Type':types[extension]||'application/octet-stream','Cache-Control':'no-store'});response.end(fs.readFileSync(file));}
http.createServer((request,response)=>{
  const url=new URL(request.url,'http://127.0.0.1:4173');
  if(url.pathname==='/desktop/'||url.pathname==='/desktop/index.html'){
    let html=fs.readFileSync(path.join(root,'desktop-p0/ui/index.html'),'utf8');
    const mock=`<script>window.__TAURI__={core:{invoke:async command=>command==='snapshot'||command==='refresh_quota'?${JSON.stringify(desktopSnapshot)}:command==='check_update'?{configured:false,available:false,current_version:'0.1.0'}:{running:false}}};</script>`;
    html=html.replace('<script src="layout.js"',mock+'<script src="layout.js"');response.writeHead(200,{'Content-Type':types['.html'],'Cache-Control':'no-store'});return response.end(html);
  }
  if(url.pathname==='/tablet/'||url.pathname==='/tablet/index.html'){
    let html=fs.readFileSync(path.join(root,'tablet-ui/index.html'),'utf8');
    const preview=`<script>addEventListener('load',()=>{render(${JSON.stringify(tabletSnapshot)},true);document.getElementById('pairing').hidden=true;document.getElementById('dashboard').hidden=false;document.getElementById('connection').className='connection online';document.getElementById('status').textContent='已連接';});</script>`;
    html=html.replace('</body>',preview+'</body>');response.writeHead(200,{'Content-Type':types['.html'],'Cache-Control':'no-store'});return response.end(html);
  }
  if(url.pathname.startsWith('/assets/')){
    const file=path.resolve(root,'tablet-ui',url.pathname.slice(1));const allowed=path.resolve(root,'tablet-ui')+path.sep;
    if(file.startsWith(allowed)&&fs.existsSync(file))return sendFile(response,file);
  }
  const match=url.pathname.match(/^\/(desktop|tablet)\/(.+)$/);if(!match){response.writeHead(404);return response.end();}
  const base=match[1]==='desktop'?'desktop-p0/ui':'tablet-ui';const file=path.resolve(root,base,match[2]);const allowed=path.resolve(root,base)+path.sep;
  if(!file.startsWith(allowed)||!fs.existsSync(file)){response.writeHead(404);return response.end();}sendFile(response,file);
}).listen(4173,'127.0.0.1',()=>console.log('AgentMeter preview http://127.0.0.1:4173/desktop/'));
