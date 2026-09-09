/* SUDOMAKE Partition — site: release e contribuidores em tempo real (API do GitHub),
   apoiantes (donors.json), galeria. Os textos estáticos já vêm traduzidos no HTML de cada
   idioma (gerado por tools/build-site.py); os textos dinâmicos vêm de i18n.json. */
(function () {
  var REPO = 'arturjose0/sudomake-partition';
  var API = 'https://api.github.com/repos/' + REPO;
  var BASE = document.documentElement.getAttribute('data-base') || '';
  var lang = (document.documentElement.lang || 'pt').slice(0, 2);
  var T = {};

  function t(k) { return (T[lang] && T[lang][k]) || (T.pt && T.pt[k]) || k; }

  function refresh() { renderRelease(); renderContributors(); renderDonors(); }

  fetch(BASE + 'i18n.json').then(function (r) { return r.ok ? r.json() : {}; })
    .then(function (j) { T = j || {}; refresh(); })
    .catch(function () { refresh(); });

  // ---- release mais recente ----
  var release = null, releaseFailed = false;
  function fmtSize(b) { if (b > 1e6) return (b / 1e6).toFixed(1) + ' MB'; if (b > 1e3) return (b / 1e3).toFixed(0) + ' KB'; return b + ' B'; }
  function renderRelease() {
    var name = document.getElementById('relName'), meta = document.getElementById('relMeta'), list = document.getElementById('assets');
    if (!release) { if (releaseFailed) { meta.textContent = t('d_fail'); } return; }
    var ver = release.tag_name.replace(/^v/, '');
    name.textContent = t('d_latest') + ': ' + ver;
    var date = new Date(release.published_at);
    var total = 0;
    (release.assets || []).forEach(function (a) { total += a.download_count || 0; });
    var loc = lang === 'en' ? 'en-US' : lang === 'fr' ? 'fr-FR' : lang === 'es' ? 'es-ES' : 'pt-PT';
    meta.textContent = t('d_published') + ' ' + date.toLocaleDateString(loc) + ' · ' + total + ' ' + t('d_downloads');
    list.innerHTML = '';
    (release.assets || []).forEach(function (a) {
      var li = document.createElement('li');
      var link = document.createElement('a'); link.href = a.browser_download_url; link.textContent = a.name;
      var n = document.createElement('span'); n.className = 'n'; n.textContent = fmtSize(a.size) + ' · ' + (a.download_count || 0) + ' ' + t('d_downloads');
      li.appendChild(link); li.appendChild(n); list.appendChild(li);
      if (/setup.*\.exe$/i.test(a.name)) { document.getElementById('btnDownload').href = a.browser_download_url; document.getElementById('dlVersion').textContent = ver; }
      if (/\.zip$/i.test(a.name)) { document.getElementById('btnZip').href = a.browser_download_url; }
    });
  }
  fetch(API + '/releases/latest', { headers: { Accept: 'application/vnd.github+json' } })
    .then(function (r) { if (!r.ok) throw new Error(r.status); return r.json(); })
    .then(function (j) { release = j; renderRelease(); })
    .catch(function () { releaseFailed = true; renderRelease(); });

  fetch(API, { headers: { Accept: 'application/vnd.github+json' } })
    .then(function (r) { return r.ok ? r.json() : null; })
    .then(function (j) { if (j && typeof j.stargazers_count === 'number') document.getElementById('stars').textContent = j.stargazers_count; })
    .catch(function () {});

  // ---- contribuidores ----
  var contributors = null, contribFailed = false;
  function renderContributors() {
    var box = document.getElementById('contributors');
    if (!contributors) { if (contribFailed) box.innerHTML = '<span class="empty">' + t('c_fail') + '</span>'; return; }
    box.innerHTML = '';
    contributors.forEach(function (c) {
      var a = document.createElement('a'); a.className = 'person'; a.href = c.html_url; a.target = '_blank'; a.rel = 'noopener';
      var img = document.createElement('img'); img.src = c.avatar_url + '&s=80'; img.alt = '';
      var d = document.createElement('div'); d.innerHTML = '<b></b><small></small>';
      d.querySelector('b').textContent = c.login; d.querySelector('small').textContent = c.contributions + ' ' + t('c_commits');
      a.appendChild(img); a.appendChild(d); box.appendChild(a);
    });
  }
  fetch(API + '/contributors?per_page=100', { headers: { Accept: 'application/vnd.github+json' } })
    .then(function (r) { if (!r.ok) throw new Error(r.status); return r.json(); })
    .then(function (j) { contributors = Array.isArray(j) ? j.filter(function (c) { return c.type !== 'Bot'; }) : []; renderContributors(); })
    .catch(function () { contribFailed = true; renderContributors(); });

  // ---- apoiantes (donors.json) ----
  var donors = null;
  function renderDonors() {
    var box = document.getElementById('donors');
    if (!donors) return;
    if (!donors.length) { box.innerHTML = '<span class="empty">' + t('g_empty') + '</span>'; return; }
    box.innerHTML = '';
    donors.slice().reverse().forEach(function (d) {
      var el = document.createElement('div'); el.className = 'donor';
      var b = document.createElement('b'); b.textContent = d.name || 'Anónimo';
      var s = document.createElement('span'); s.textContent = [d.country, d.date, d.amount].filter(Boolean).join(' · ');
      el.appendChild(b); el.appendChild(s);
      if (d.message) { var em = document.createElement('em'); em.textContent = '“' + d.message + '”'; el.appendChild(em); }
      box.appendChild(el);
    });
    var th = document.createElement('p'); th.className = 'meta'; th.textContent = t('g_thanks'); th.style.gridColumn = '1 / -1'; box.appendChild(th);
  }
  fetch(BASE + 'donors.json?' + Date.now()).then(function (r) { return r.ok ? r.json() : { donors: [] }; })
    .then(function (j) { donors = (j && j.donors) || []; renderDonors(); })
    .catch(function () { donors = []; renderDonors(); });

  // ---- galeria ----
  var lb = document.getElementById('lightbox');
  document.querySelectorAll('.shot-zoom').forEach(function (img) {
    img.addEventListener('click', function () { lb.querySelector('img').src = img.src; lb.classList.add('on'); });
  });
  lb.addEventListener('click', function () { lb.classList.remove('on'); });
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape') lb.classList.remove('on'); });

  // ---- menu, FAQ ----
  var menu = document.getElementById('menu');
  document.getElementById('menuToggle').addEventListener('click', function () { menu.classList.toggle('open'); });
  menu.addEventListener('click', function () { menu.classList.remove('open'); });
  document.getElementById('year').textContent = new Date().getFullYear();
})();
