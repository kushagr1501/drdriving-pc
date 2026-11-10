// Runs before WASM creates its context: one profile controls resolution, shader and LOD.
var drDisplay=(function(){
  var key='drdriving.graphics.v1',stored='auto';
  try{stored=localStorage.getItem(key)||'auto';}catch(e){}
  if(!/^(auto|performance|high)$/.test(stored))stored='auto';
  var coarse=!!(window.matchMedia&&window.matchMedia('(pointer:coarse)').matches);
  var mobile=coarse||(/Android|iPhone|iPad|iPod/i.test(navigator.userAgent))||(navigator.platform==='MacIntel'&&navigator.maxTouchPoints>1);
  var lightweight=stored==='performance'||(stored==='auto'&&mobile);
  // Macroquad's JS backend creates the WebGL context without passing Rust's
  // sample_count. Set the attributes on this canvas before that first call.
  var canvas=document.getElementById('glcanvas'),getContext=canvas.getContext;
  canvas.getContext=function(type,attributes){
    if(lightweight&&(type==='webgl'||type==='webgl2'||type==='experimental-webgl'))attributes=Object.assign({},attributes,{antialias:false});
    return getContext.call(canvas,type,attributes);
  };
  var panel=document.getElementById('display-settings'),close=document.getElementById('display-close');
  var quality=document.getElementById('graphics-quality'),apply=document.getElementById('quality-apply');
  var button=document.getElementById('fs'),root=document.documentElement,previousFocus;
  var request=root.requestFullscreen||root.webkitRequestFullscreen;
  var available=!!request&&document.fullscreenEnabled!==false&&document.webkitFullscreenEnabled!==false;
  var standalone=!!navigator.standalone||!!(window.matchMedia&&window.matchMedia('(display-mode:standalone)').matches);
  quality.value=stored;
  var description=document.getElementById('quality-note');
  function describe(){
    description.textContent=quality.value==='high'?'Full car detail and Retina resolution. Uses more battery.':quality.value==='performance'?'Lighter car meshes, reflections and resolution for smoother driving.':mobile?'Auto uses Performance on this device.':'Auto uses High on this device.';
    apply.hidden=quality.value===stored;
  }
  describe();quality.addEventListener('change',describe);
  function open(){previousFocus=document.activeElement;panel.hidden=false;close.focus({preventScroll:true});}
  function dismiss(){panel.hidden=true;(previousFocus||document.getElementById('glcanvas')).focus({preventScroll:true});}
  close.addEventListener('click',dismiss);
  panel.addEventListener('click',function(e){if(e.target===panel)dismiss();});
  document.getElementById('graphics').addEventListener('click',open);
  document.addEventListener('keydown',function(e){
    if(panel.hidden)return;
    // Keep settings keystrokes out of the driving controls.
    e.stopPropagation();
    if(e.key==='Escape'){e.preventDefault();dismiss();}
    if(e.key==='Tab'){
      var items=Array.from(panel.querySelectorAll('button,select')).filter(function(el){return !el.hidden;});
      var first=items[0],last=items[items.length-1];
      if(e.shiftKey&&document.activeElement===first){e.preventDefault();last.focus();}
      else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first.focus();}
    }
  },true);
  apply.addEventListener('click',function(){
    try{localStorage.setItem(key,quality.value);location.reload();}
    catch(e){description.textContent='Allow site storage to save the graphics setting.';}
  });
  var help=document.getElementById('screen-help'),steps=document.getElementById('screen-steps');
  help.textContent=standalone?'Opened from your Home Screen. Turn your phone sideways for the widest view.':available?'Use the fullscreen button to hide the browser bars. Turn your phone sideways for the widest view.':'For a larger view, open this game from your Home Screen. Turn your phone sideways to drive.';
  steps.hidden=available||standalone;
  if(!available&&!standalone&&/Android/i.test(navigator.userAgent))document.getElementById('install-step').textContent='Open your browser menu and choose “Install app” or “Add to Home screen”.';
  function isFullscreen(){return !!(document.fullscreenElement||document.webkitFullscreenElement);}
  function focusCanvas(){document.getElementById('glcanvas').focus({preventScroll:true});}
  function entered(){
    // Orientation locking is optional and may reject on Safari or desktop.
    try{var lock=screen.orientation.lock('landscape');if(lock&&lock.catch)lock.catch(function(){});}catch(e){}
    focusCanvas();
  }
  button.title=available?'Fullscreen':'Screen options';
  button.setAttribute('aria-label',button.title);
  button.addEventListener('click',function(e){
    e.preventDefault();
    if(!available){open();return;}
    try{
      if(isFullscreen()){
        var exit=document.exitFullscreen||document.webkitExitFullscreen;
        var result=exit.call(document);if(result&&result.catch)result.catch(open);
        focusCanvas();
      }else{
        var result=request.call(root);if(result&&result.then)result.then(entered,open);else entered();
      }
    }catch(error){open();}
  });
  function screenChanged(){button.setAttribute('aria-label',isFullscreen()?'Exit fullscreen':available?'Fullscreen':'Screen options');}
  document.addEventListener('fullscreenchange',screenChanged);
  document.addEventListener('webkitfullscreenchange',screenChanged);
  document.getElementById('screen-tools').addEventListener('touchstart',function(e){e.stopPropagation();},{passive:true});
  document.addEventListener('contextmenu',function(e){e.preventDefault();});
  // Safari's visible height changes as its address bar opens/closes. Macroquad
  // reads canvas.clientWidth/clientHeight and rect offsets, keeping touch aligned.
  function sizeViewport(){
    var height=window.visualViewport?window.visualViewport.height:window.innerHeight;
    root.style.setProperty('--app-height',height+'px');
  }
  sizeViewport();window.addEventListener('resize',sizeViewport);
  if(window.visualViewport)window.visualViewport.addEventListener('resize',sizeViewport);
  if(window.ResizeObserver)new ResizeObserver(function(){if(typeof window.onresize==='function')window.onresize();}).observe(document.getElementById('game-viewport'));
  return {lightweight:lightweight,active:function(){return document.getElementById('welcome').style.display==='none'&&!document.hidden;}};
})();
