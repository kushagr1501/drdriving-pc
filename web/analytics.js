// Plain-HTML Vercel Web Analytics. The game remains on GitHub Pages; this
// dedicated Vercel project supplies the script and documented custom endpoint.
(function(){
  if(location.hostname!=='dr-driving.meowsy.space')return;
  if(navigator.globalPrivacyControl||navigator.doNotTrack==='1'||window.doNotTrack==='1')return;
  if(document.getElementById('dr-vercel-analytics'))return;
  window.va=window.va||function(){(window.vaq=window.vaq||[]).push(arguments);};
  window.va('beforeSend',function(event){
    // Keep diagnostic query values and fragments out of the recorded page URL.
    try{var url=new URL(event.url);url.search='';url.hash='';return Object.assign({},event,{url:url.href});}
    catch(error){return null;}
  });
  var script=document.createElement('script');
  script.id='dr-vercel-analytics';
  script.src='https://dr-driving-analytics.vercel.app/collect/script.js';
  script.dataset.viewEndpoint='https://dr-driving-analytics.vercel.app/collect/view';
  script.async=true;
  script.defer=true;
  // A blocked analytics request must never surface as a game-start failure.
  script.onerror=function(){};
  document.head.appendChild(script);
})();
