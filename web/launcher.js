var drLauncher=(function(){
  var welcome=document.getElementById('welcome');
  var play=document.getElementById('play');
  var label=document.getElementById('play-label');
  var progress=document.getElementById('download-progress');
  var note=document.getElementById('load-note');
  var state=document.getElementById('ready-state');
  var ready=false,lastUpdate=0;
  var tabs=[document.getElementById('keyboard-tab'),document.getElementById('touch-tab')];
  play.disabled=true;
  function selectMode(mode){
    tabs.forEach(function(tab){var selected=tab.id===mode+'-tab';tab.setAttribute('aria-selected',String(selected));tab.tabIndex=selected?0:-1;});
    document.getElementById('keyboard-panel').hidden=mode!=='keyboard';
    document.getElementById('touch-panel').hidden=mode!=='touch';
  }
  tabs.forEach(function(tab){tab.addEventListener('click',function(){selectMode(tab.id==='touch-tab'?'touch':'keyboard');});tab.addEventListener('keydown',function(event){if(event.key==='ArrowRight'||event.key==='ArrowLeft'){event.preventDefault();var next=tab===tabs[0]?tabs[1]:tabs[0];next.click();next.focus();}});});
  selectMode(window.matchMedia&&window.matchMedia('(pointer:coarse)').matches?'touch':'keyboard');
  play.addEventListener('click',function(){if(!ready)return;welcome.style.display='none';document.getElementById('glcanvas').focus();});
  document.addEventListener('keydown',function(event){
    if(welcome.style.display==='none'||event.ctrlKey||event.metaKey||event.altKey)return;
    var key=event.key.toLowerCase();if(key===' ')key='space';
    if(!/^(w|a|s|d|arrowup|arrowdown|arrowleft|arrowright|[1-4]|c|l|v|space|escape|enter)$/.test(key))return;
    document.querySelectorAll('[data-key~="'+key+'"]').forEach(function(cap){cap.classList.add('pressed');});
    if(event.target.closest&&event.target.closest('button,a'))return;
    event.preventDefault();event.stopPropagation();
    if(key==='enter'&&ready)play.click();
  },true);
  document.addEventListener('keyup',function(event){if(welcome.style.display==='none')return;var key=event.key.toLowerCase();if(key===' ')key='space';if(/^[a-z0-9]+$/.test(key))document.querySelectorAll('[data-key~="'+key+'"]').forEach(function(cap){cap.classList.remove('pressed');});},true);
  return {
    downloading:function(loaded,total){if(ready)return;var now=Date.now();if(now-lastUpdate<200&&loaded<total)return;lastUpdate=now;note.textContent='Downloading game files';state.textContent=(loaded/1048576).toFixed(1)+(total?' / '+(total/1048576).toFixed(1):'')+' MB';if(total)progress.value=Math.min(100,loaded/total*100);},
    preparing:function(){if(ready)return;note.textContent='Preparing cars and audio';state.textContent='Almost there';progress.removeAttribute('value');},
    ready:function(){ready=true;play.disabled=false;label.textContent='PLAY GAME';progress.value=100;note.textContent='Ready to drive';state.textContent='● READY';state.dataset.ready='true';if(document.activeElement===document.body||document.activeElement===document.getElementById('glcanvas'))play.focus();}
  };
})();
