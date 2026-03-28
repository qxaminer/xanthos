/* Shared navigation — xanthos.dev
   Inject with: <div id="nav-root"></div><script src="/nav.js"></script>
   For pages inside subdirectories, use: <script src="/nav.js" data-root=".."></script>
*/
(function () {
  const root = (function () {
    const el = document.currentScript;
    return el && el.getAttribute('data-root') ? el.getAttribute('data-root') : '';
  })();

  const p = (href) => root ? root + '/' + href : href;

  const nav = document.getElementById('nav-root');
  if (!nav) return;

  nav.innerHTML =
    '<nav class="xnav">' +
      '<a href="' + p('index.html') + '">Xanthos</a>' +
      '<span class="xnav-sep"></span>' +
      '<a href="' + p('work/index.html') + '">Work</a>' +
      '<a href="' + p('demos/index.html') + '">Demos</a>' +
      '<a href="' + p('about/index.html') + '">About</a>' +
    '</nav>';

  const style = document.createElement('style');
  style.textContent = [
    '.xnav {',
    '  display: flex;',
    '  justify-content: center;',
    '  align-items: center;',
    '  gap: 6px;',
    '  padding: 18px 20px 10px;',
    '  flex-wrap: wrap;',
    '}',
    '.xnav a {',
    '  color: #D4AF37;',
    '  text-decoration: none;',
    '  font-size: 0.95em;',
    '  letter-spacing: 0.4px;',
    '  font-family: "Minion Pro", Georgia, serif;',
    '}',
    '.xnav a:first-child {',
    '  font-size: 1.05em;',
    '  letter-spacing: 0.8px;',
    '}',
    '.xnav a:hover { color: #CE4F1A; }',
    '.xnav-sep {',
    '  width: 1px;',
    '  height: 14px;',
    '  background: rgba(212, 175, 55, 0.3);',
    '  margin: 0 10px;',
    '}',
  ].join('\n');
  document.head.appendChild(style);
})();
