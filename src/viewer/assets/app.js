'use strict';
// Tala viewer. Every value from the store is escaped before it reaches the DOM.
const TOKEN = new URLSearchParams(location.hash.slice(1)).get('t') || '';
const S = {
  tab: 'feed', thread: null, head: '', meta: null, A: {},
  feed: {msgs: [], more: false}, view: null, seen: 0, timer: null, busy: false,
};
const $ = s => document.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'}[c]));
const base = p => p ? p.split('/').filter(Boolean).pop() : '';

class Unauthorised extends Error {}
async function api(path, params = {}) {
  const q = new URLSearchParams(Object.entries(params).filter(([, v]) => v !== '' && v != null));
  const r = await fetch(path + (q.toString() ? '?' + q : ''), {headers: {Authorization: 'Bearer ' + TOKEN}, cache: 'no-store'});
  if (r.status === 401) throw new Unauthorised();
  if (!r.ok) throw new Error('HTTP ' + r.status);
  return r.json();
}
const filters = () => ({project: $('#fProject').value, agent: $('#fAgent').value, q: $('#fText').value.trim()});

// Formatting
const fmtT = s => new Date(s * 1000).toLocaleTimeString([], {hour: '2-digit', minute: '2-digit'});
function fmtD(s) {
  const d = new Date(s * 1000), now = new Date(S.meta.now * 1000), y = new Date(now);
  y.setDate(now.getDate() - 1);
  if (d.toDateString() === now.toDateString()) return 'Today';
  if (d.toDateString() === y.toDateString()) return 'Yesterday';
  return d.toLocaleDateString([], {weekday: 'long', day: 'numeric', month: 'long'});
}
function ago(s) {
  const d = S.meta.now - s;
  if (d < 60) return 'now';
  if (d < 3600) return Math.floor(d / 60) + 'm';
  if (d < 86400) return Math.floor(d / 3600) + 'h';
  return Math.floor(d / 86400) + 'd';
}
function hue(id) { let h = 0; for (const c of id) h = (h * 31 + c.charCodeAt(0)) >>> 0; return h % 360; }

function indexAgents() {
  const names = {};
  S.meta.agents.forEach(a => names[a.name] = (names[a.name] || 0) + 1);
  S.A = {};
  S.meta.agents.forEach(a => {
    a.label = names[a.name] > 1 ? `${a.name}·${a.id.slice(4, 8)}` : a.name;
    a.proj = base(a.project);
    let rest = a.name;
    if (a.proj && rest.toLowerCase().startsWith(a.proj.toLowerCase() + '-')) rest = rest.slice(a.proj.length + 1);
    const parts = rest.split(/[-_\s.]+/).filter(Boolean);
    a.ini = (parts.length > 1 ? parts[0][0] + parts[1][0] : (parts[0] || '?').slice(0, 2)).toUpperCase();
    a.hue = hue(a.id);
    S.A[a.id] = a;
  });
}

// Pieces
const av = (id, sm) => { const a = S.A[id]; return `<span class="av ${sm ? 'sm' : ''}" style="--h:${a ? a.hue : 0}" title="${esc(a?.label || id)}">${esc(a?.ini || '?')}</span>`; };
const nm = id => {
  const a = S.A[id];
  return a ? `<b class="link" data-agent="${esc(id)}" title="${esc(a.tool)}${a.branch ? ' · ' + esc(a.branch) : ''}">${esc(a.label)}</b>` : `<b>${esc(id)}</b>`;
};
function state(m) {
  if (m.state) {
    const cls = {answered: 'ok', declined: 'bad', cancelled: '', accepted: 'acc', open: 'warn'}[m.state];
    const label = {open: 'awaiting reply', accepted: 'accepted, in progress'}[m.state] || m.state;
    return `<span class="st ${cls}">${esc(label)}</span>`;
  }
  if (m.overdue) return '<span class="st bad">unread, overdue</span>';
  if (m.unread) return '<span class="st">unread</span>';
  return '';
}
function item(m, o = {}) {
  const to = m.recipient ? nm(m.recipient) : `<b>${esc(base(m.project))} board</b>`;
  const foot = [];
  if (!o.inThread && m.project) foot.push(esc(base(m.project)));
  if (m.reply_to) foot.push(`reply to <span class="id">#${esc(m.reply_to)}</span>`);
  if (!o.inThread) foot.push(`<a data-thread="${esc(m.thread)}">thread</a>`);
  foot.push(`<span class="id">#${esc(m.id)}</span>`);
  return `<article class="item ${S.seen && m.id > S.seen ? 'new' : ''}">${av(m.sender)}
    <div class="c"><div class="l1">${nm(m.sender)}${o.inThread ? '' : `<span class="to">to</span>${to}`} ${state(m)}<time>${fmtT(m.created_at)}</time></div>
    <div class="text">${esc(m.text)}</div><div class="foot">${foot.map(f => `<span>${f}</span>`).join('')}</div></div></article>`;
}
function days(ms, o) {
  let out = '', day = '';
  ms.forEach(m => { const d = fmtD(m.created_at); if (d !== day) { out += `<div class="day">${esc(d)}</div>`; day = d; } out += item(m, o); });
  return out;
}
const tbl = (rows, f) => rows.length ? `<table>${rows.map(f).join('')}</table>` : '<div class="empty">Nothing here.</div>';
const n = v => v ? esc(v) : '<span class="zero">–</span>';
const pairCell = (a, verb, b) => `<div class="ag"><div class="stack">${av(a, true)}${av(b, true)}</div><div>${nm(a)} <span class="sub">${verb}</span> ${nm(b)}</div></div>`;
const snip = m => `<a class="snip" data-thread="${esc(m.thread)}">${esc(m.text.slice(0, 160))}</a>`;

// Views
const views = {
  async feed(keep) {
    const limit = keep ? Math.min(Math.max(S.feed.msgs.length, 200), 1000) : 200;
    const r = await api('/api/messages', {...filters(), limit});
    S.feed = {msgs: r.messages, more: r.more};
    return renderFeed;
  },
  async convos() {
    const list = await api('/api/threads', filters());
    if (!S.thread || !list.threads.some(t => t.thread === S.thread)) S.thread = list.threads[0]?.thread || null;
    const tx = S.thread ? await api('/api/threads/' + encodeURIComponent(S.thread)) : null;
    S.view = {list, tx};
    return renderConvos;
  },
  async agents() { S.view = await api('/api/agents', filters()); return renderAgents; },
  async attention() { S.view = await api('/api/attention', filters()); return renderAttention; },
};

function renderFeed() {
  const ms = S.feed.msgs;
  if (!ms.length) return '<div class="note">No messages match.</div>';
  return days(ms) + (S.feed.more ? '<button class="more" id="older">Load older messages</button>' : '');
}
function renderConvos() {
  const {list, tx} = S.view;
  const rows = list.threads.map(t => {
    const title = t.board ? `${base(t.project)} board` : t.participants.map(i => S.A[i]?.label || i).join(', ');
    return `<div class="row ${t.thread === S.thread ? 'on' : ''}" data-thread="${esc(t.thread)}">
      <div class="stack">${t.participants.slice(0, 2).map(i => av(i, true)).join('')}</div>
      <div class="c"><div class="who"><b>${esc(title)}</b><time>${ago(t.latest_at)}</time></div>
      <div class="ex">${esc(t.excerpt)}</div>
      <div class="meta"><span>${t.count} message${t.count > 1 ? 's' : ''}</span>${t.waiting ? '<span style="color:var(--warn)">awaiting reply</span>' : ''}</div></div></div>`;
  }).join('') + (list.more ? '<div class="note">Showing the 100 most recent conversations. Filter to narrow them.</div>' : '');
  let right = '<div class="note">No conversation selected.</div>';
  if (tx && tx.messages.length) {
    const who = [...new Set(tx.messages.flatMap(m => [m.sender, m.recipient]).filter(Boolean))];
    const board = tx.messages[0].kind === 'post';
    const proj = base(tx.messages[0].project);
    right = `<div class="tx-h"><div class="stack">${who.map(i => av(i)).join('')}</div><div><h2>${board ? esc(proj) + ' board' : who.map(nm).join(' <span style="color:var(--faint);font-weight:400">and</span> ')}</h2>
      <span>${esc(proj)} · ${tx.messages.length} message${tx.messages.length > 1 ? 's' : ''}</span></div></div>${days(tx.messages, {inThread: true})}`;
  }
  return `<div class="split"><div class="list">${rows || '<div class="note">No conversations.</div>'}</div><div class="tx">${right}</div></div>`;
}
function renderAgents() {
  const rows = S.view.agents.map(s => {
    const a = S.A[s.id] || {id: s.id};
    return `<tr><td><div class="ag"><span class="dot ${a.listening ? 'listen' : a.recent ? 'recent' : ''}" title="${a.listening ? 'listening' : a.recent ? 'recently active' : ''}"></span>${av(s.id, true)}<div>${nm(s.id)}<div class="sub">${esc(a.tool)}</div></div></div></td>
      <td class="hide-s">${esc(a.proj)}</td><td class="hide-s mono sub">${esc(a.branch || '')}</td>
      <td class="num">${n(s.sent)}</td><td class="num">${n(s.received)}</td><td class="num">${n(s.unread)}</td><td class="num">${n(s.owes)}</td>
      <td class="num sub">${a.last_seen ? ago(a.last_seen) : ''}</td></tr>`;
  }).join('');
  const max = S.view.pairs[0]?.count || 1;
  const pairs = S.view.pairs.map(p => { const [x, y] = p.agents;
    return `<tr><td><div class="ag"><div class="stack">${av(x, true)}${av(y, true)}</div><div>${nm(x)} <span class="sub">and</span> ${nm(y)}</div></div></td>
      <td style="width:34%"><div class="meter"><i style="width:${(p.count / max * 100).toFixed(1)}%"></i></div></td><td class="num" style="width:50px">${esc(p.count)}</td></tr>`; }).join('');
  return `<h2 class="sec">Agents <span><span style="color:var(--ok)">●</span> listening · <span style="color:var(--warn)">●</span> active in the last 15 minutes</span></h2>
    <table><tr><th>Agent</th><th class="hide-s">Project</th><th class="hide-s">Branch</th><th class="num">Sent</th><th class="num">Received</th><th class="num">Unread</th><th class="num">Owes reply</th><th class="num">Seen</th></tr>${rows}</table>
    <h2 class="sec">Who talks to whom <span>direct messages, both directions</span></h2>${pairs ? `<table>${pairs}</table>` : '<div class="empty">Nothing here.</div>'}`;
}
function renderAttention() {
  const v = S.view;
  const reqRow = m => `<tr><td style="width:36%">${pairCell(m.recipient, 'owes', m.sender)}</td><td>${snip(m)}</td><td style="width:150px">${state(m)}</td><td class="num sub" style="width:44px">${ago(m.created_at)}</td></tr>`;
  const unreadRow = m => `<tr><td style="width:36%">${pairCell(m.sender, 'to', m.recipient)}</td><td>${snip(m)}</td><td style="width:150px"><span class="st ${m.recipient_listening ? 'ok' : 'bad'}">${m.recipient_listening ? 'listening' : 'not listening'}</span></td><td class="num sub" style="width:44px">${ago(m.created_at)}</td></tr>`;
  return `<h2 class="sec">Awaiting a reply <span>recipient active in the last day</span></h2>${tbl(v.awaiting, reqRow)}
    <h2 class="sec">Unread <span>recipient active in the last day${v.quiet_unread ? ` · ${esc(v.quiet_unread)} more to agents that have gone quiet` : ''}</span></h2>${tbl(v.unread, unreadRow)}
    <h2 class="sec">Abandoned requests <span>recipient not seen for over a day</span></h2>${tbl(v.abandoned, reqRow)}`;
}

// Chrome
function renderTabs() {
  const c = S.meta?.attention_count || 0;
  const tabs = [['feed', 'Feed'], ['convos', 'Conversations'], ['agents', 'Agents'], ['attention', 'Attention']];
  $('#tabs').innerHTML = tabs.map(([k, l]) => `<button class="${S.tab === k ? 'on' : ''}" data-tab="${k}">${l}${k === 'attention' && c ? `<span class="n">${esc(c)}</span>` : ''}</button>`).join('');
}
function fillFilters() {
  const p = $('#fProject').value, a = $('#fAgent').value;
  $('#fProject').innerHTML = '<option value="">All projects</option>' + S.meta.projects.map(x => `<option value="${esc(x)}" ${x === p ? 'selected' : ''}>${esc(base(x))}</option>`).join('');
  const ags = S.meta.agents.slice().sort((x, y) => x.label.localeCompare(y.label));
  $('#fAgent').innerHTML = '<option value="">All agents</option>' + ags.map(x => `<option value="${esc(x.id)}" ${x.id === a ? 'selected' : ''}>${esc(x.label)}</option>`).join('');
}
function setLive(ok, text) {
  const el = $('#live');
  el.className = 'live' + (ok ? '' : ' off');
  el.innerHTML = `<i></i><span>${esc(text)}</span>`;
}
function fail(e) {
  if (e instanceof Unauthorised) {
    clearInterval(S.timer);
    setLive(false, 'not authorised');
    $('#main').innerHTML = '<div class="note">This link is not valid for the running viewer. Open the URL that tala ui printed.</div>';
  } else setLive(false, 'disconnected');
}

async function refresh(keep) {
  if (S.busy) return;
  S.busy = true;
  try {
    const [head, meta] = await Promise.all([api('/api/head'), api('/api/meta')]);
    S.meta = meta; indexAgents(); fillFilters(); renderTabs();
    const render = await views[S.tab](keep);
    const y = scrollY;
    $('#main').innerHTML = render();
    if (keep) scrollTo(0, y);
    if (S.tab === 'feed') S.seen = Math.max(S.seen, S.feed.msgs[0]?.id || 0);
    S.head = head.head;
    setLive(true, meta.home);
  } catch (e) { fail(e); } finally { S.busy = false; }
}
async function poll() {
  try { const {head} = await api('/api/head'); if (head !== S.head) await refresh(true); else setLive(true, S.meta?.home || ''); }
  catch (e) { fail(e); }
}
async function older(btn) {
  btn.disabled = true;
  try {
    const r = await api('/api/messages', {...filters(), before: S.feed.msgs.at(-1).id, limit: 200});
    S.feed.msgs = S.feed.msgs.concat(r.messages);
    S.feed.more = r.more;
    const y = scrollY;
    $('#main').innerHTML = renderFeed();
    scrollTo(0, y);
  } catch (e) { fail(e); btn.disabled = false; }
}

document.addEventListener('click', e => {
  const el = e.target.closest('[data-tab],[data-thread],[data-agent],#older,.text');
  if (!el) return;
  if (el.id === 'older') return older(el);
  if (el.dataset.tab) { S.tab = el.dataset.tab; scrollTo(0, 0); return refresh(false); }
  if (el.dataset.thread) { S.thread = el.dataset.thread; if (S.tab !== 'convos') { S.tab = 'convos'; scrollTo(0, 0); } return refresh(S.tab === 'convos'); }
  if (el.dataset.agent) { $('#fAgent').value = el.dataset.agent; return refresh(false); }
  if (!getSelection().toString()) el.classList.toggle('open');
});
['#fProject', '#fAgent'].forEach(s => $(s).addEventListener('change', () => refresh(false)));
let typing;
$('#fText').addEventListener('input', () => { clearTimeout(typing); typing = setTimeout(() => refresh(false), 250); });

if (!TOKEN) {
  setLive(false, 'no token');
  $('#main').innerHTML = '<div class="note">Missing access token. Open the URL that tala ui printed.</div>';
} else {
  renderTabs();
  refresh(false).then(() => { S.timer = setInterval(poll, 2000); });
}
