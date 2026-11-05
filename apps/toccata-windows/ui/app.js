const invoke = window.__TAURI__ && window.__TAURI__.core
  ? window.__TAURI__.core.invoke
  : null;

const TASK_VIEWS = ['tasks', 'inbox', 'review'];

const VIEW_META = {
  tasks: { title: '任务' },
  inbox: {
    title: '收集箱',
    sub: '想到就记下。什么时候做，由你决定。'
  },
  calendar: {
    title: '任务日历',
    sub: '按月查看任务的创建、计划与完成日期。'
  },
  review: {
    title: '任务回顾',
    sub: '回看这段时间完成的事，也带上还没做完的任务。'
  },
  activity: {
    title: '键盘活跃',
    sub: '记录键盘活动，看见自己的思维净活跃时长。'
  },
  ranking: {
    title: '应用与趋势',
    sub: '按活跃时长查看常用应用与变化趋势。'
  }
};

const KEYBOARD_ROWS = [
  [
    { codes: [0x1B], label: 'esc' },
    { spacer: 0.5 },
    { codes: [0x70], label: 'F1' }, { codes: [0x71], label: 'F2' },
    { codes: [0x72], label: 'F3' }, { codes: [0x73], label: 'F4' },
    { spacer: 0.2 },
    { codes: [0x74], label: 'F5' }, { codes: [0x75], label: 'F6' },
    { codes: [0x76], label: 'F7' }, { codes: [0x77], label: 'F8' },
    { spacer: 0.2 },
    { codes: [0x78], label: 'F9' }, { codes: [0x79], label: 'F10' },
    { codes: [0x7A], label: 'F11' }, { codes: [0x7B], label: 'F12' }
  ],
  [
    { codes: [0xC0], label: '`' }, { codes: [0x31], label: '1' },
    { codes: [0x32], label: '2' }, { codes: [0x33], label: '3' },
    { codes: [0x34], label: '4' }, { codes: [0x35], label: '5' },
    { codes: [0x36], label: '6' }, { codes: [0x37], label: '7' },
    { codes: [0x38], label: '8' }, { codes: [0x39], label: '9' },
    { codes: [0x30], label: '0' }, { codes: [0xBD], label: '-' },
    { codes: [0xBB], label: '=' }, { codes: [0x08], label: '⌫', w: 2 }
  ],
  [
    { codes: [0x09], label: 'Tab', w: 1.5 },
    { codes: [0x51], label: 'Q' }, { codes: [0x57], label: 'W' },
    { codes: [0x45], label: 'E' }, { codes: [0x52], label: 'R' },
    { codes: [0x54], label: 'T' }, { codes: [0x59], label: 'Y' },
    { codes: [0x55], label: 'U' }, { codes: [0x49], label: 'I' },
    { codes: [0x4F], label: 'O' }, { codes: [0x50], label: 'P' },
    { codes: [0xDB], label: '[' }, { codes: [0xDD], label: ']' },
    { codes: [0xDC], label: '\\', w: 1.5 }
  ],
  [
    { codes: [0x14], label: 'Caps', w: 1.75 },
    { codes: [0x41], label: 'A' }, { codes: [0x53], label: 'S' },
    { codes: [0x44], label: 'D' }, { codes: [0x46], label: 'F' },
    { codes: [0x47], label: 'G' }, { codes: [0x48], label: 'H' },
    { codes: [0x4A], label: 'J' }, { codes: [0x4B], label: 'K' },
    { codes: [0x4C], label: 'L' }, { codes: [0xBA], label: ';' },
    { codes: [0xDE], label: "'" }, { codes: [0x0D], label: 'Enter', w: 2.25 }
  ],
  [
    { codes: [0xA0, 0xA1], label: 'Shift', w: 2.25 },
    { codes: [0x5A], label: 'Z' }, { codes: [0x58], label: 'X' },
    { codes: [0x43], label: 'C' }, { codes: [0x56], label: 'V' },
    { codes: [0x42], label: 'B' }, { codes: [0x4E], label: 'N' },
    { codes: [0x4D], label: 'M' }, { codes: [0xBC], label: ',' },
    { codes: [0xBE], label: '.' }, { codes: [0xBF], label: '/' },
    { codes: [0xA0, 0xA1], label: 'Shift', w: 2.75 }
  ],
  [
    { codes: [0xA2, 0xA3], label: 'Ctrl', w: 1.25 },
    { codes: [0x5B, 0x5C], label: 'Win', w: 1.25 },
    { codes: [0xA4, 0xA5], label: 'Alt', w: 1.25 },
    { codes: [0x20], label: 'Space', w: 6.25 },
    { codes: [0xA4, 0xA5], label: 'Alt', w: 1.25 },
    { codes: [0x5B, 0x5C], label: 'Win', w: 1.25 },
    { codes: [0x5D], label: 'Menu', w: 1.25 },
    { codes: [0xA2, 0xA3], label: 'Ctrl', w: 1.25 }
  ]
];

const APP_NAMES = {
  'code.exe': 'VS Code',
  'chrome.exe': 'Google Chrome',
  'msedge.exe': 'Microsoft Edge',
  'firefox.exe': 'Firefox',
  'explorer.exe': '资源管理器',
  'windowsterminal.exe': 'Windows Terminal',
  'powershell.exe': 'PowerShell',
  'cmd.exe': '命令提示符',
  'idea64.exe': 'IntelliJ IDEA',
  'pycharm64.exe': 'PyCharm',
  'wechat.exe': '微信',
  'qq.exe': 'QQ',
  'dingtalk.exe': '钉钉',
  'feishu.exe': '飞书',
  'lark.exe': '飞书',
  'steam.exe': 'Steam',
  'discord.exe': 'Discord',
  'spotify.exe': 'Spotify'
};

const DEFAULT_REMINDER_SETTINGS = {
  defaultEnabled: false,
  leadMinutes: 10,
  silentFullscreen: true,
  dateOnlyTime: '09:00'
};

function loadReminderSettings() {
  try {
    const raw = JSON.parse(localStorage.getItem('toccata.reminderSettings') || 'null');
    if (!raw || typeof raw !== 'object') return { ...DEFAULT_REMINDER_SETTINGS };
    const lead = Number(raw.leadMinutes);
    return {
      defaultEnabled: Boolean(raw.defaultEnabled),
      leadMinutes: Number.isFinite(lead) ? Math.min(10080, Math.max(0, Math.round(lead))) : DEFAULT_REMINDER_SETTINGS.leadMinutes,
      silentFullscreen: raw.silentFullscreen !== false,
      dateOnlyTime: /^\d{1,2}:\d{2}$/.test(raw.dateOnlyTime || '') ? raw.dateOnlyTime : DEFAULT_REMINDER_SETTINGS.dateOnlyTime
    };
  } catch {
    return { ...DEFAULT_REMINDER_SETTINGS };
  }
}

const state = {
  tasks: [],
  byId: new Map(),
  children: new Map(),
  view: 'tasks',
  period: 'week',
  query: '',
  display: readSetting('toccata.listDisplay', 'titles'),
  sort: readSetting('toccata.sortOrder', 'manual'),
  selectedId: null,
  collapsed: new Set(),
  showCompleted: true,
  detailsEditing: false,
  dirty: false,
  saveState: 'saved',
  saveTimer: null,
  snapshot: null,
  reminderSettings: loadReminderSettings(),
  closeMode: readCloseMode(),
  closePromptOpen: false,
  reminderQueue: [],
  reminderShowing: false,
  reminderTaskId: null,
  firedReminders: new Set(),
  calendarMonth: startOfMonth(new Date()),
  calendarDay: startOfDay(new Date()),
  activityRange: 'today',
  rankingRange: 'today',
  activityData: null,
  rankingData: null,
  rankingDaily: null,
  activityLoading: false,
  rankingLoading: false,
  captureEnabled: null
};

const tasksArea = document.getElementById('tasks-area');
const calendarPane = document.getElementById('calendar-pane');
const dataPane = document.getElementById('data-pane');
const listEl = document.getElementById('task-list');
const emptyEl = document.getElementById('empty');
const detailEl = document.getElementById('detail');
const detailEmptyEl = document.getElementById('detail-empty');
const titleEl = document.getElementById('edit-title');
const detailsEl = document.getElementById('edit-details');
const previewEl = document.getElementById('details-preview');
const dateEl = document.getElementById('edit-date');
const timeEl = document.getElementById('edit-time');
const subtaskListEl = document.getElementById('subtask-list');
const searchEl = document.getElementById('search');

/* ---------- settings ---------- */

function readSetting(key, fallback) {
  try {
    return localStorage.getItem(key) || fallback;
  } catch {
    return fallback;
  }
}

function readCloseMode() {
  const value = readSetting('toccata.closeBehavior', 'ask');
  return ['minimize', 'exit', 'ask'].includes(value) ? value : 'ask';
}

function writeSetting(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* Settings are optional. */
  }
}

/* ---------- time ---------- */

function nowSeconds() {
  return Date.now() / 1000;
}

function startOfDay(date) {
  const d = new Date(date);
  d.setHours(0, 0, 0, 0);
  return d;
}

function startOfMonth(date) {
  return new Date(date.getFullYear(), date.getMonth(), 1);
}

function addMonths(date, delta) {
  return new Date(date.getFullYear(), date.getMonth() + delta, 1);
}

function addDays(date, delta) {
  const d = new Date(date);
  d.setDate(d.getDate() + delta);
  return d;
}

function startOfDaySeconds(date) {
  return startOfDay(date).getTime() / 1000;
}

function weekStartSeconds() {
  const d = startOfDay(new Date());
  const offset = (d.getDay() + 6) % 7;
  d.setDate(d.getDate() - offset);
  return d.getTime() / 1000;
}

function periodStart(period) {
  return period === 'week'
    ? weekStartSeconds()
    : startOfDaySeconds(addDays(new Date(), -6));
}

function formatDate(seconds) {
  if (seconds == null) return '';
  return new Intl.DateTimeFormat('zh-CN', { month: 'numeric', day: 'numeric' })
    .format(new Date(seconds * 1000));
}

function formatLongDate(seconds) {
  if (seconds == null) return '';
  const d = new Date(seconds * 1000);
  return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日`;
}

function formatTime(seconds) {
  if (seconds == null) return '';
  return new Intl.DateTimeFormat('zh-CN', {
    hour: '2-digit', minute: '2-digit', hour12: false
  }).format(new Date(seconds * 1000));
}

function formatFull(seconds) {
  if (seconds == null) return '';
  return `${formatLongDate(seconds)} ${formatTime(seconds)}`;
}

function formatClock(seconds, withSeconds = false) {
  if (seconds == null) return '';
  const d = new Date(seconds * 1000);
  const base = `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
  return withSeconds && d.getSeconds() > 0
    ? `${base}:${String(d.getSeconds()).padStart(2, '0')}`
    : base;
}

function timeInputValue(seconds) {
  if (seconds == null) return '';
  const d = new Date(seconds * 1000);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}:${String(d.getSeconds()).padStart(2, '0')}`;
}

function dateWithTime(dateSeconds, hhmm) {
  if (dateSeconds == null) return null;
  const [hour, minute] = String(hhmm || '09:00').split(':').map(Number);
  const d = new Date(dateSeconds * 1000);
  d.setHours(hour || 0, minute || 0, 0, 0);
  return d.getTime() / 1000;
}

function remainingInfo(plannedAt, now = nowSeconds()) {
  const diff = plannedAt - now;
  const overdue = diff < 0;
  const abs = Math.abs(diff);
  const plannedDay = startOfDay(new Date(plannedAt * 1000));
  const today = startOfDay(new Date(now * 1000));
  const dayDiff = Math.round((plannedDay.getTime() - today.getTime()) / 86400000);

  if (dayDiff >= 1 && abs >= 86400) {
    return { text: `${dayDiff}天`, tone: 'far' };
  }
  if (dayDiff <= -1 && abs >= 86400) {
    return { text: `逾期${-dayDiff}天`, tone: 'soon' };
  }

  const prefix = overdue ? '逾期' : '';
  if (abs >= 3600) {
    const hours = Math.floor(abs / 3600);
    const minutes = Math.floor((abs % 3600) / 60);
    return { text: `${prefix}${hours}小时${minutes > 0 ? `${minutes}分` : ''}`, tone: 'soon' };
  }
  if (abs >= 60) {
    const minutes = Math.floor(abs / 60);
    const seconds = Math.floor(abs % 60);
    return { text: `${prefix}${minutes}分${seconds > 0 ? `${seconds}秒` : ''}`, tone: 'soon' };
  }
  return { text: `${prefix}${Math.max(0, Math.floor(abs))}秒`, tone: 'soon' };
}

function updateRemainingBadges() {
  const now = nowSeconds();
  document.querySelectorAll('.row-remaining[data-planned]').forEach(element => {
    const plannedAt = Number(element.dataset.planned);
    if (!Number.isFinite(plannedAt)) return;
    const info = remainingInfo(plannedAt, now);
    element.textContent = info.text;
    element.className = `row-remaining ${info.tone}`;
  });
}

function reminderTimeFor(task) {
  if (!task || task.plannedAt == null) return null;
  const base = task.plannedHasTime
    ? task.plannedAt
    : dateWithTime(task.plannedAt, state.reminderSettings.dateOnlyTime);
  const lead = task.reminderLeadMinutes == null
    ? state.reminderSettings.leadMinutes
    : task.reminderLeadMinutes;
  return base - Math.max(0, lead) * 60;
}

function weekdayLabel(date) {
  return new Intl.DateTimeFormat('zh-CN', { weekday: 'short' }).format(date);
}

function dayKeyFromDate(date) {
  const d = new Date(date);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

function dayKeyFromSeconds(seconds) {
  return seconds == null ? null : dayKeyFromDate(new Date(seconds * 1000));
}

function todayKey() {
  return dayKeyFromDate(new Date());
}

function dateInputValue(seconds) {
  return seconds == null ? '' : dayKeyFromSeconds(seconds);
}

function dateInputSeconds(value) {
  if (!value) return null;
  const [y, m, d] = value.split('-').map(Number);
  if (!y || !m || !d) return null;
  return new Date(y, m - 1, d, 0, 0, 0, 0).getTime() / 1000;
}

/* ---------- natural language input ---------- */

const CHINESE_HOURS = {
  零: 0, 一: 1, 二: 2, 两: 2, 三: 3, 四: 4, 五: 5, 六: 6, 七: 7, 八: 8, 九: 9,
  十: 10, 十一: 11, 十二: 12
};

function chineseNumber(value) {
  if (/^\d+$/.test(value)) return Number(value);
  if (value in CHINESE_HOURS) return CHINESE_HOURS[value];
  if (value.includes('十')) {
    const [left, right] = value.split('十');
    const tens = left ? (CHINESE_HOURS[left] || 0) : 1;
    const ones = right ? (CHINESE_HOURS[right] || 0) : 0;
    return tens * 10 + ones;
  }
  return CHINESE_HOURS[value] ?? NaN;
}

function resolveAmbiguousHour(hour, period, now) {
  if (period) {
    if (period === '凌晨') return hour === 12 ? 0 : hour;
    if (period === '中午') return (hour >= 1 && hour < 12) ? hour + 12 : hour;
    if (period === '下午' || period === '傍晚') return hour < 12 ? hour + 12 : hour;
    if (period === '晚上') return hour === 12 ? 0 : (hour < 12 ? hour + 12 : hour);
    return hour;
  }
  if (hour === 12) return 12;
  if (hour >= 5 && hour <= 11 && now.getHours() >= hour) return hour + 12;
  return hour;
}

function parseTaskInput(raw, now = new Date()) {
  const ranges = [];
  let text = String(raw || '').trim()
    .replace(/今晚/g, '今天晚上')
    .replace(/明晚/g, '明天晚上')
    .replace(/今早/g, '今天早上')
    .replace(/明早/g, '明天早上');
  let date = null;
  let explicitDate = false;
  let time = null;

  const consume = match => {
    if (match && match.index != null) {
      ranges.push([match.index, match.index + match[0].length]);
    }
  };

  const fullDate = text.match(/(20\d{2})\s*[.\-/年]\s*(\d{1,2})\s*[.\-/月]\s*(\d{1,2})\s*[日号]?/);
  if (fullDate) {
    const [y, m, d] = [Number(fullDate[1]), Number(fullDate[2]), Number(fullDate[3])];
    const candidate = new Date(y, m - 1, d);
    if (candidate.getFullYear() === y && candidate.getMonth() === m - 1 && candidate.getDate() === d) {
      date = startOfDay(candidate);
      explicitDate = true;
      consume(fullDate);
    }
  }

  if (!date) {
    const monthDay = text.match(/(?<!\d)(\d{1,2})\s*[.\-/月]\s*(\d{1,2})\s*[日号]?/);
    if (monthDay) {
      const [m, d] = [Number(monthDay[1]), Number(monthDay[2])];
      const candidate = new Date(now.getFullYear(), m - 1, d);
      if (candidate.getMonth() === m - 1 && candidate.getDate() === d) {
        date = startOfDay(candidate);
        explicitDate = true;
        consume(monthDay);
      }
    }
  }

  if (!date) {
    const relative = text.match(/大后天|后天|明天|明日|今天|今日|昨天|昨日/);
    if (relative) {
      const offset = { 大后天: 3, 后天: 2, 明天: 1, 明日: 1, 今天: 0, 今日: 0, 昨天: -1, 昨日: -1 }[relative[0]];
      date = startOfDay(addDays(now, offset));
      explicitDate = true;
      consume(relative);
    }
  }

  if (!date) {
    const weekday = text.match(/(这|本|下|上)?\s*(?:周|星期|礼拜)([一二三四五六日天])/);
    if (weekday) {
      const target = { 一: 0, 二: 1, 三: 2, 四: 3, 五: 4, 六: 5, 日: 6, 天: 6 }[weekday[2]];
      const today = startOfDay(now);
      const currentIndex = (today.getDay() + 6) % 7;
      const monday = addDays(today, -currentIndex);
      if (weekday[1] === '这' || weekday[1] === '本') {
        date = startOfDay(addDays(monday, target));
      } else if (weekday[1] === '下') {
        date = startOfDay(addDays(monday, 7 + target));
      } else if (weekday[1] === '上') {
        date = startOfDay(addDays(monday, -7 + target));
      } else {
        date = startOfDay(addDays(today, (target - currentIndex + 7) % 7));
      }
      explicitDate = true;
      consume(weekday);
    }
  }

  if (!date) {
    const dayOnly = text.match(/(?<!\d)(\d{1,2})\s*[日号]/);
    if (dayOnly) {
      const d = Number(dayOnly[1]);
      const candidate = new Date(now.getFullYear(), now.getMonth(), d);
      if (candidate.getDate() === d) {
        date = startOfDay(candidate);
        explicitDate = true;
        consume(dayOnly);
      }
    }
  }

  const clock = text.match(/(凌晨|早上|上午|中午|下午|傍晚|晚上)?\s*(\d{1,2}):(\d{1,2})(?::(\d{1,2}))?(?!\d)/);
  if (clock) {
    const period = clock[1] || null;
    const rawHour = Number(clock[2]);
    const minute = Number(clock[3]);
    const second = clock[4] ? Number(clock[4]) : 0;
    if (rawHour <= 23 && minute <= 59 && second <= 59) {
      time = {
        hour: period ? resolveAmbiguousHour(rawHour, period, now) : rawHour,
        minute,
        second
      };
      consume(clock);
    }
  }

  if (!time) {
    const spoken = text.match(/(凌晨|早上|上午|中午|下午|傍晚|晚上)?\s*([0-9]{1,2}|[零一二两三四五六七八九十]{1,3})\s*(?:点|时)(?:\s*(半|一刻|三刻|([0-9]{1,2})\s*分?))?/);
    if (spoken) {
      const period = spoken[1] || null;
      const rawHour = chineseNumber(spoken[2]);
      let minute = 0;
      if (spoken[3] === '半') minute = 30;
      else if (spoken[3] === '一刻') minute = 15;
      else if (spoken[3] === '三刻') minute = 45;
      else if (spoken[4]) minute = Number(spoken[4]);
      if (Number.isFinite(rawHour) && rawHour >= 0 && rawHour <= 23 && minute <= 59) {
        time = {
          hour: resolveAmbiguousHour(rawHour, period, now),
          minute,
          second: 0
        };
        consume(spoken);
      }
    }
  }

  if (!date && !time) {
    return { title: text.trim() || '新任务', date: null, hasTime: false };
  }

  if (!date) {
    date = startOfDay(now);
    const candidate = new Date(date);
    candidate.setHours(time.hour, time.minute, time.second, 0);
    if (candidate.getTime() <= now.getTime()) {
      date = addDays(date, 1);
    }
  }

  const result = startOfDay(date);
  if (time) {
    result.setHours(time.hour, time.minute, time.second, 0);
  }

  // A time-only expression already rolled forward above.
  if (!explicitDate && time) {
    // Keep the rolled date from the time-only branch.
  }

  ranges
    .sort((a, b) => b[0] - a[0])
    .forEach(([start, end]) => {
      text = `${text.slice(0, start)} ${text.slice(end)}`;
    });
  let title = text
    .replace(/\s+/g, ' ')
    .replace(/^[\s,，。.;；:：、\-—]+|[\s,，。.;；:：、\-—]+$/g, '')
    .trim();
  if (!title) title = '新任务';

  return { title, date: result, hasTime: Boolean(time) };
}

/* ---------- data ---------- */

function indexTasks() {
  state.byId = new Map(state.tasks.map(task => [task.id, task]));
  state.children = new Map();
  for (const task of state.tasks) {
    if (!task.parentId) continue;
    if (!state.children.has(task.parentId)) state.children.set(task.parentId, []);
    state.children.get(task.parentId).push(task);
  }
  for (const children of state.children.values()) children.sort(compareTasks);
}

function childrenOf(id) {
  return state.children.get(id) || [];
}

function rootOf(task) {
  let current = task;
  const seen = new Set();
  while (current && current.parentId && !seen.has(current.id)) {
    seen.add(current.id);
    const parent = state.byId.get(current.parentId);
    if (!parent) break;
    current = parent;
  }
  return current || task;
}

function compareTasks(a, b) {
  const pinnedA = a.isPinned ? 0 : 1;
  const pinnedB = b.isPinned ? 0 : 1;
  if (pinnedA !== pinnedB) return pinnedA - pinnedB;

  switch (state.sort) {
    case 'created-asc':
      return a.createdAt - b.createdAt || a.title.localeCompare(b.title, 'zh-Hans-CN');
    case 'created-desc':
      return b.createdAt - a.createdAt || a.title.localeCompare(b.title, 'zh-Hans-CN');
    case 'title':
      return a.title.localeCompare(b.title, 'zh-Hans-CN') || b.createdAt - a.createdAt;
    case 'planned': {
      const plannedA = a.plannedAt == null ? Infinity : a.plannedAt;
      const plannedB = b.plannedAt == null ? Infinity : b.plannedAt;
      return plannedA - plannedB || b.createdAt - a.createdAt;
    }
    case 'manual':
    default: {
      const orderA = a.subtaskOrder;
      const orderB = b.subtaskOrder;
      if (orderA == null && orderB == null) return b.createdAt - a.createdAt;
      if (orderA == null) return -1;
      if (orderB == null) return 1;
      if (orderA !== orderB) return orderA - orderB;
      return b.createdAt - a.createdAt;
    }
  }
}

function inPeriod(task, period, now = nowSeconds()) {
  if (task.createdAt > now) return false;
  const start = periodStart(period);
  if (period === 'recent') return task.createdAt >= start;
  if (task.createdAt >= start) return true;
  if (!task.completedAt && task.createdAt < start) return true;
  return task.completedAt != null && task.completedAt >= start && task.completedAt <= now;
}

function completedInPeriod(task, period) {
  return task.completedAt != null
    && task.completedAt >= periodStart(period)
    && task.completedAt <= nowSeconds();
}

function scopedTasks() {
  if (state.view === 'inbox') return state.tasks;
  if (state.view === 'tasks') {
    return state.tasks.filter(task => inPeriod(task, state.period));
  }
  if (state.view === 'review') {
    const start = periodStart(state.period);
    const now = nowSeconds();
    return state.tasks.filter(task =>
      task.completedAt == null
      || (task.completedAt >= start && task.completedAt <= now));
  }
  return [];
}

function matchesQuery(task) {
  if (!state.query.trim()) return true;
  const needle = state.query.trim().toLowerCase();
  return task.title.toLowerCase().includes(needle)
    || (task.details || '').toLowerCase().includes(needle);
}

function visibleTaskIds() {
  const scoped = scopedTasks();
  const scopedIds = new Set(scoped.map(task => task.id));
  if (!state.query.trim()) return { ids: scopedIds, scopedCount: scoped.length };

  const ids = new Set();
  for (const task of scoped) {
    if (!matchesQuery(task)) continue;
    ids.add(task.id);
    let parentId = task.parentId;
    while (parentId && scopedIds.has(parentId)) {
      ids.add(parentId);
      const parent = state.byId.get(parentId);
      parentId = parent ? parent.parentId : null;
    }
  }
  return { ids, scopedCount: scoped.length };
}

/* ---------- render: sidebar & header ---------- */

function selectedTask() {
  return state.selectedId ? state.byId.get(state.selectedId) : null;
}

function renderSidebar() {
  document.querySelectorAll('.nav-item').forEach(button => {
    button.classList.toggle('active', button.dataset.view === state.view);
  });
  const openRoots = state.tasks.filter(task => !task.parentId && !task.completedAt).length;
  document.getElementById('inbox-count').textContent = openRoots > 0 ? String(openRoots) : '';
}

function sublineForView() {
  if (state.view === 'tasks') {
    return state.period === 'week'
      ? '本周新建、之前未完成，以及本周完成的任务。'
      : '今天及之前六个自然日创建的任务。';
  }
  return VIEW_META[state.view].sub;
}

function renderHeader() {
  const isTaskView = TASK_VIEWS.includes(state.view);
  document.getElementById('view-title').textContent = VIEW_META[state.view].title;
  document.getElementById('view-sub').textContent = sublineForView();

  document.getElementById('copy-review').classList.toggle('hidden', state.view !== 'review');
  document.getElementById('period-control').classList.toggle('hidden', !['tasks', 'review'].includes(state.view));
  document.getElementById('quick-add').classList.toggle('hidden', !['tasks', 'inbox'].includes(state.view));
  document.getElementById('review-summary').classList.toggle('hidden', state.view !== 'review');

  document.querySelectorAll('#period-control button').forEach(button => {
    button.classList.toggle('active', button.dataset.period === state.period);
  });

  if (state.view === 'review') {
    const completed = state.tasks.filter(task => completedInPeriod(task, state.period)).length;
    const pending = state.tasks.filter(task => !task.completedAt).length;
    document.getElementById('review-summary').innerHTML =
      `<strong>已完成 ${completed} 项</strong> · 未完成 ${pending} 项<br>复制回顾会包含完成状态、任务详情与子任务。`;
  }

  if (searchEl.value !== state.query) searchEl.value = state.query;
  document.getElementById('clear-search').classList.toggle('hidden', !state.query);

  document.getElementById('display-label').textContent = state.display === 'details' ? '含详情' : '仅标题';
  document.getElementById('sort-select').value = state.sort;

  const foot = document.getElementById('list-foot');
  if (state.view === 'review') {
    foot.innerHTML = '<svg class="icon"><use href="#i-doc"></use></svg><span>按主任务归组；已完成子任务保留在所属任务下</span>';
  } else {
    foot.innerHTML = '<svg class="icon"><use href="#i-checklist"></use></svg><span>视图按时间组织任务，不会自动设置计划日期</span>';
  }

  const item = document.querySelector(`.nav-item[data-view="${state.view}"]`);
  if (item) item.scrollIntoView({ block: 'nearest' });
}

/* ---------- render: task list ---------- */

function flattenRows(roots, visibleIds) {
  const collapsed = state.query.trim() ? new Set() : state.collapsed;
  const rows = [];
  const push = (task, depth) => {
    rows.push({ task, depth });
    if (collapsed.has(task.id)) return;
    const children = childrenOf(task.id)
      .filter(child => visibleIds.has(child.id))
      .sort(compareTasks);
    for (const child of children) {
      if (depth < 2) push(child, depth + 1);
    }
  };
  for (const root of roots) push(root, 0);
  return rows;
}

function rowHtml(task, depth, visibleIds) {
  const done = task.completedAt != null;
  const selected = state.selectedId === task.id;
  const children = childrenOf(task.id).filter(child => visibleIds.has(child.id));
  const collapsed = (state.query.trim() ? new Set() : state.collapsed).has(task.id);
  const showDetails = state.display === 'details' && task.details.trim();
  const parentHint = depth === 0 && task.parentId && !visibleIds.has(task.parentId)
    ? state.byId.get(task.parentId)
    : null;
  const pinned = !task.parentId && task.isPinned;
  const remaining = !done && task.plannedAt != null ? remainingInfo(task.plannedAt) : null;

  return `
    <div class="row-wrap ${depth > 0 ? 'child' : ''}" data-id="${escapeHtml(task.id)}" style="--depth:${depth}">
      <div class="task-row ${selected ? 'selected' : ''}">
        <button class="row-check ${done ? 'done' : ''}" title="${done ? '恢复任务' : '完成任务'}" aria-label="${done ? '恢复任务' : '完成任务'}"></button>
        <div class="row-body">
          <div class="row-title ${done ? 'done' : ''}">${escapeHtml(task.title)}</div>
          ${parentHint ? `<div class="row-details">父任务：${escapeHtml(parentHint.title)}</div>` : ''}
          ${showDetails ? `<div class="row-details">${escapeHtml(task.details)}</div>` : ''}
        </div>
        ${remaining ? `<span class="row-remaining ${remaining.tone}" data-planned="${task.plannedAt}">${remaining.text}</span>` : ''}
        ${pinned
          ? '<svg class="icon pin-mark" aria-label="已置顶"><use href="#i-pin"></use></svg>'
          : '<span class="pin-slot" aria-hidden="true"></span>'}
        ${children.length
          ? `<button class="row-expand" title="${collapsed ? '展开子任务' : '收起子任务'}"><svg class="icon"><use href="${collapsed ? '#i-chevron-right' : '#i-chevron-down'}"></use></svg></button>`
          : '<span class="row-expand-spacer" aria-hidden="true"></span>'}
        <svg class="icon grip" aria-hidden="true"><use href="#i-grip"></use></svg>
      </div>
    </div>`;
}

function sectionHeaderHtml(title, collapsible, collapsed) {
  if (!collapsible) {
    return `<div class="section-header">${escapeHtml(title)}</div>`;
  }
  return `
    <button class="section-header ${collapsed ? 'collapsed' : ''}" data-section="completed">
      <svg class="icon"><use href="#i-chevron-down"></use></svg>
      <span>${escapeHtml(title)}</span>
    </button>`;
}

function renderList() {
  if (!TASK_VIEWS.includes(state.view)) return;

  const { ids, scopedCount } = visibleTaskIds();
  const visible = state.tasks.filter(task => ids.has(task.id));
  const roots = visible
    .filter(task => !task.parentId || !ids.has(task.parentId))
    .sort(compareTasks);
  const pendingRoots = roots.filter(task => !task.completedAt);
  const completedRoots = roots.filter(task => task.completedAt);

  const pendingRows = flattenRows(pendingRoots, ids);
  const completedRows = flattenRows(completedRoots, ids);
  const hasRows = pendingRows.length + completedRows.length > 0;

  if (!hasRows) {
    listEl.innerHTML = '';
    emptyEl.classList.remove('hidden');
    const searching = state.query.trim().length > 0;
    const emptyTitle = document.getElementById('empty-title');
    const emptyText = document.getElementById('empty-text');
    if (searching) {
      emptyTitle.textContent = '没有匹配的任务';
      emptyText.textContent = '试试其他关键词。';
    } else if (state.view === 'inbox') {
      emptyTitle.textContent = '先收集，再安排';
      emptyText.textContent = '把想到的事情放进来。没有计划日期，也可以安心待在这里。';
    } else if (state.view === 'review') {
      emptyTitle.textContent = '这段时间还没有任务可回顾';
      emptyText.textContent = '本期完成的任务和当前未完成的事项都会出现在这里，方便整理周报。';
    } else {
      emptyTitle.textContent = '这个视图还没有任务';
      emptyText.textContent = scopedCount === 0
        ? '添加一个新任务，或切换范围查看其他记录。'
        : '调整搜索条件试试。';
    }
    return;
  }

  emptyEl.classList.add('hidden');

  const isReview = state.view === 'review';
  let html = '';

  if (isReview) {
    html += sectionHeaderHtml(`已完成主任务 · ${completedRoots.length} 组`, false, false);
    html += completedRows.map(row => rowHtml(row.task, row.depth, ids)).join('');
    html += sectionHeaderHtml(`待推进主任务 · ${pendingRoots.length} 组`, false, false);
    html += pendingRows.map(row => rowHtml(row.task, row.depth, ids)).join('');
  } else {
    html += sectionHeaderHtml(`待办 · ${pendingRoots.length} 项主任务`, false, false);
    html += pendingRows.map(row => rowHtml(row.task, row.depth, ids)).join('');
    html += sectionHeaderHtml(`已完成 · ${completedRoots.length} 项主任务`, true, !state.showCompleted);
    if (state.showCompleted) {
      html += completedRows.map(row => rowHtml(row.task, row.depth, ids)).join('');
    }
  }

  listEl.innerHTML = html;
}

/* ---------- render: detail ---------- */

function autoGrowTitle() {
  titleEl.style.height = 'auto';
  titleEl.style.height = `${Math.min(Math.max(titleEl.scrollHeight, 30), 132)}px`;
}

function updateSaveState() {
  const el = document.getElementById('save-state');
  if (!el) return;
  let text = '已保存';
  if (state.saveState === 'invalid') text = '标题不能为空';
  else if (state.saveState === 'error') text = '保存失败，请重试';
  else if (state.dirty) text = '正在保存…';
  el.textContent = text;
  el.classList.toggle('warn', state.saveState === 'invalid' || state.saveState === 'error');
}

function updateDetailChrome() {
  const task = selectedTask();
  const hasTask = Boolean(task);
  detailEl.classList.toggle('hidden', !hasTask);
  detailEmptyEl.classList.toggle('hidden', hasTask);
  if (!task) return;

  const parent = task.parentId ? state.byId.get(task.parentId) : null;
  document.getElementById('back-parent').classList.toggle('hidden', !parent);
  document.getElementById('detail-label').classList.toggle('hidden', Boolean(parent));
  if (parent) document.getElementById('back-title').textContent = parent.title;

  const root = rootOf(task);
  const pinButton = document.getElementById('pin-btn');
  pinButton.classList.toggle('pinned', root.isPinned);
  pinButton.title = root.isPinned ? '取消置顶' : '置顶任务';

  const done = task.completedAt != null;
  const ring = document.getElementById('complete-ring');
  ring.classList.toggle('done', done);
  document.getElementById('complete-label').textContent = done ? '已完成 · 点击恢复待办' : '标记完成';
  document.getElementById('completed-at').textContent = done ? `完成于 ${formatFull(task.completedAt)}` : '';

  const hasDate = task.plannedAt != null;
  const hasTime = hasDate && task.plannedHasTime;
  const popoverOpen = !document.getElementById('date-popover').classList.contains('hidden');
  if (!popoverOpen) {
    dateEl.value = hasDate ? dayKeyFromSeconds(task.plannedAt) : '';
    timeEl.value = hasTime ? timeInputValue(task.plannedAt) : '';
  }
  document.getElementById('planned-label').textContent = hasDate
    ? `${formatLongDate(task.plannedAt)}${hasTime ? ` ${formatClock(task.plannedAt, true)}` : ''}`
    : '添加计划日期';
  document.getElementById('planned-sub').textContent = hasDate
    ? `${weekdayLabel(new Date(task.plannedAt * 1000))} · ${hasTime ? '具体时间' : '仅日期'} · 点击调整`
    : '可选择日期与具体时间';

  const reminderToggle = document.getElementById('reminder-toggle');
  reminderToggle.checked = Boolean(task.reminderEnabled);
  reminderToggle.disabled = !hasDate && !task.reminderEnabled;
  const leadValue = task.reminderLeadMinutes == null
    ? state.reminderSettings.leadMinutes
    : task.reminderLeadMinutes;
  document.getElementById('reminder-lead').value = String(leadValue);
  const reminderAt = reminderTimeFor(task);
  document.getElementById('reminder-at').textContent = !hasDate
    ? '设置日期后可提醒'
    : !task.reminderEnabled
      ? '未开启提醒'
      : `提醒于 ${formatFull(reminderAt)}`;

  const children = childrenOf(task.id).sort(compareTasks);
  const completedChildren = children.filter(child => child.completedAt).length;
  document.getElementById('subtask-count').textContent = children.length
    ? `${completedChildren} / ${children.length}`
    : '';

  subtaskListEl.innerHTML = children.map(child => {
    const childDone = child.completedAt != null;
    const detail = (child.details || '').trim();
    return `
      <div class="subtask-row" data-id="${escapeHtml(child.id)}">
        <button class="row-check ${childDone ? 'done' : ''}" title="${childDone ? '恢复子任务' : '完成子任务'}" aria-label="${childDone ? '恢复子任务' : '完成子任务'}"></button>
        <div class="subtask-body">
          <div class="subtask-title ${childDone ? 'done' : ''}">${escapeHtml(child.title)}</div>
          ${detail ? `<div class="subtask-detail">${escapeHtml(detail)}</div>` : ''}
        </div>
        <svg class="icon grip" aria-hidden="true"><use href="#i-grip"></use></svg>
      </div>`;
  }).join('');

  const isSubtask = Boolean(task.parentId);
  document.getElementById('subtask-add').classList.toggle('hidden', isSubtask);
  document.getElementById('subtask-note').classList.toggle('hidden', !isSubtask);

  const metadata = [`创建于 ${formatFull(task.createdAt)}`];
  if (task.completedAt != null) metadata.push(`完成于 ${formatFull(task.completedAt)}`);
  document.getElementById('metadata').innerHTML = metadata.map(escapeHtml).join('<br>');

  updateSaveState();
  autoGrowTitle();
}

function fillEditor(force = true) {
  const task = selectedTask();
  if (!task) {
    updateDetailChrome();
    return;
  }
  if (force || document.activeElement !== titleEl) titleEl.value = task.title;
  if (force || document.activeElement !== detailsEl) detailsEl.value = task.details || '';
  autoGrowTitle();

  state.detailsEditing = false;
  renderDetailsMode();

  state.snapshot = {
    title: task.title,
    details: task.details || '',
    plannedAt: task.plannedAt,
    plannedHasTime: task.plannedHasTime,
    reminderEnabled: task.reminderEnabled,
    reminderLeadMinutes: task.reminderLeadMinutes
  };
  state.dirty = false;
  state.saveState = 'saved';
  closeDatePopover();
  updateDetailChrome();
}

function renderDetailsMode() {
  const hasText = detailsEl.value.trim().length > 0;
  const showEditor = state.detailsEditing || !hasText;
  detailsEl.classList.toggle('hidden', !showEditor);
  previewEl.classList.toggle('hidden', showEditor);
  const button = document.getElementById('preview-btn');
  button.textContent = showEditor ? '预览' : '编辑';
  button.disabled = showEditor && !hasText;
  if (!showEditor) {
    previewEl.innerHTML = markdownToHtml(detailsEl.value) || '<p style="color:#b0b0b5">暂无内容。</p>';
  }
}

function renderDetail() {
  if (!TASK_VIEWS.includes(state.view)) return;
  updateDetailChrome();
}

/* ---------- render: calendar ---------- */

function tasksOnDay(day) {
  const key = dayKeyFromDate(day);
  const entries = [];
  for (const task of state.tasks) {
    const labels = [];
    if (dayKeyFromSeconds(task.createdAt) === key) labels.push('created');
    if (dayKeyFromSeconds(task.plannedAt) === key) labels.push('planned');
    if (dayKeyFromSeconds(task.completedAt) === key) labels.push('completed');
    if (labels.length) entries.push({ task, labels });
  }
  entries.sort((a, b) => a.task.title.localeCompare(b.task.title, 'zh-Hans-CN'));
  return entries;
}

function renderCalendar() {
  const month = state.calendarMonth;
  const year = month.getFullYear();
  const monthIndex = month.getMonth();
  document.getElementById('cal-month').textContent = `${year}年${monthIndex + 1}月`;

  const first = new Date(year, monthIndex, 1);
  const offset = (first.getDay() + 6) % 7;
  const gridStart = new Date(year, monthIndex, 1 - offset);
  const today = todayKey();
  const selectedKey = dayKeyFromDate(state.calendarDay);
  const cells = [];

  for (let i = 0; i < 42; i += 1) {
    const date = addDays(gridStart, i);
    const key = dayKeyFromDate(date);
    const outside = date.getMonth() !== monthIndex;
    const entries = tasksOnDay(date);
    const kinds = new Set();
    for (const entry of entries) for (const label of entry.labels) kinds.add(label);
    const overdue = state.tasks.some(task =>
      task.plannedAt != null
      && !task.completedAt
      && task.plannedAt < nowSeconds()
      && dayKeyFromSeconds(task.plannedAt) === key);
    const classes = ['day-cell'];
    if (outside) classes.push('outside');
    if (key === today) classes.push('today');
    if (key === selectedKey) classes.push('selected');
    if (overdue) classes.push('overdue');
    cells.push(`
      <button type="button" class="${classes.join(' ')}" data-date="${key}" title="${entries.length} 项任务${overdue ? ' · 有逾期未完成任务' : ''}">
        <span class="day-number">${date.getDate()}</span>
        <span class="day-dots">${[...kinds].slice(0, 3).map(kind => `<i class="dot ${kind}"></i>`).join('')}</span>
      </button>`);
  }
  document.getElementById('calendar-grid').innerHTML = cells.join('');

  const entries = tasksOnDay(state.calendarDay);
  document.getElementById('day-title').textContent =
    `${formatLongDate(startOfDaySeconds(state.calendarDay))} ${weekdayLabel(state.calendarDay)}`;
  document.getElementById('day-count').textContent = entries.length ? `${entries.length} 项` : '';
  const dayTasks = document.getElementById('day-tasks');
  if (!entries.length) {
    dayTasks.innerHTML = '<div class="day-empty">这一天没有任务记录。</div>';
    return;
  }
  dayTasks.innerHTML = entries.map(({ task, labels }) => {
    const parent = task.parentId ? state.byId.get(task.parentId) : null;
    const chips = labels.map(label => {
      const text = label === 'created' ? '创建' : label === 'planned' ? '计划' : '完成';
      return `<span class="event-chip ${label}">${text}</span>`;
    }).join('');
    return `
      <div class="day-task" data-id="${escapeHtml(task.id)}">
        <div class="day-task-body">
          <div class="day-task-title">${escapeHtml(task.title)}</div>
          <div class="day-task-meta">
            ${chips}
            ${parent ? `<span class="event-chip">${escapeHtml(parent.title)}</span>` : ''}
          </div>
        </div>
      </div>`;
  }).join('');
}

/* ---------- keyboard activity ---------- */

function weekStartDate(date) {
  const d = startOfDay(date);
  d.setDate(d.getDate() - ((d.getDay() + 6) % 7));
  return d;
}

function dayRangesForRange(range) {
  const today = startOfDay(new Date());
  const now = Date.now();
  if (range === 'today') {
    return [{ startMs: today.getTime(), endMs: now }];
  }
  if (range === 'yesterday') {
    return [{ startMs: addDays(today, -1).getTime(), endMs: today.getTime() }];
  }
  const start = range === 'week' ? weekStartDate(today) : addDays(today, -6);
  const days = [];
  let cursor = new Date(start);
  while (cursor.getTime() <= today.getTime()) {
    const next = addDays(cursor, 1);
    days.push({
      startMs: cursor.getTime(),
      endMs: Math.min(next.getTime(), now)
    });
    cursor = next;
    if (cursor.getTime() > today.getTime()) break;
  }
  return days;
}

function activityRangeLabel(range) {
  return { today: '今日', yesterday: '昨日', last7: '最近 7 天', week: '本周' }[range] || '今日';
}

function rangeDateLabel(range) {
  const days = dayRangesForRange(range);
  if (!days.length) return '';
  const first = formatLongDate(days[0].startMs / 1000);
  const last = formatLongDate(days[days.length - 1].startMs / 1000);
  return first === last ? first : `${first} — ${last}`;
}

function formatDurationMs(ms) {
  const total = Math.max(0, Math.round(ms / 1000));
  if (total < 60) return `${total} 秒`;
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  if (hours > 0) return `${hours} 小时 ${minutes} 分`;
  return `${minutes} 分 ${seconds} 秒`;
}

function rangeButtons(kind, current) {
  return ['today', 'yesterday', 'last7', 'week'].map(range => {
    const label = activityRangeLabel(range);
    return `<button data-${kind}-range="${range}" class="${range === current ? 'active' : ''}">${label}</button>`;
  }).join('');
}

function renderKeyboardKeys(keyCounts) {
  const counts = keyCounts || {};
  let max = 1;
  for (const value of Object.values(counts)) max = Math.max(max, Number(value) || 0);

  const rows = KEYBOARD_ROWS.map(row => {
    const keys = row.map(key => {
      if (key.spacer) {
        return `<span class="kb-spacer" style="flex:${key.spacer}"></span>`;
      }
      const count = (key.codes || []).reduce((sum, code) => sum + (Number(counts[String(code)]) || 0), 0);
      const ratio = count > 0 ? count / max : 0;
      const background = count > 0
        ? `background:rgba(0,122,255,${(0.1 + ratio * 0.82).toFixed(3)});color:${ratio > 0.55 ? '#fff' : '#1d1d1f'}`
        : '';
      return `<div class="kb-key" style="flex:${key.w || 1};${background}" title="${escapeHtml(key.label)} · ${count} 次">
        <span class="kb-label">${escapeHtml(key.label)}</span>
        ${count > 0 ? `<span class="kb-hit">${count}</span>` : ''}
      </div>`;
    }).join('');
    return `<div class="kb-row">${keys}</div>`;
  }).join('');

  return `<div class="keyboard-heatmap">${rows}</div>`;
}

function renderTimeline(days) {
  if (!days || !days.length) {
    return '<div class="data-empty">这段时间还没有键盘活动。</div>';
  }
  const rows = days.map(day => {
    const date = new Date(day.startMs);
    const isToday = dayKeyFromDate(date) === todayKey();
    const isYesterday = dayKeyFromDate(date) === dayKeyFromDate(addDays(new Date(), -1));
    const label = `${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
    const weekday = isToday ? '今天' : isYesterday ? '昨天' : weekdayLabel(date);
    const buckets = day.buckets || [];
    let cells = '';
    for (let index = 0; index < 96; index += 1) {
      const bucket = buckets[index];
      if (!bucket) {
        cells += '<div class="timeline-cell empty" title="无记录"></div>';
        continue;
      }
      const span = Math.max(1, bucket.endMs - bucket.startMs);
      const fraction = Math.min(1, bucket.activeMs / span);
      const start = new Date(bucket.startMs);
      const end = new Date(bucket.endMs);
      const clock = value => `${String(value.getHours()).padStart(2, '0')}:${String(value.getMinutes()).padStart(2, '0')}`;
      const title = `${clock(start)}–${clock(end)} · 活跃 ${formatDurationMs(bucket.activeMs)} · ${bucket.keyCount} 次`;
      const background = fraction > 0
        ? `background:rgba(0,122,255,${(0.1 + fraction * 0.82).toFixed(3)})`
        : '';
      cells += `<div class="timeline-cell" style="${background}" title="${title}"></div>`;
    }
    return `
      <div class="timeline-row">
        <div class="timeline-day"><strong>${label}</strong><span>${weekday}</span></div>
        <div class="timeline-cells">${cells}</div>
        <div class="timeline-summary">
          <strong>${formatDurationMs(day.activeMs)}</strong>
          <span>${day.keyCount} 次按键</span>
        </div>
      </div>`;
  }).join('');

  return `
    <div class="timeline">
      <div class="timeline-head">
        <span></span>
        <div class="timeline-hours"><span>00:00</span><span>06:00</span><span>12:00</span><span>18:00</span><span>24:00</span></div>
        <span class="timeline-summary-head">活跃 / 按键</span>
      </div>
      ${rows}
    </div>`;
}

function captureChipHtml() {
  const stateText = state.captureEnabled === null
    ? '正在检查采集状态…'
    : state.captureEnabled
      ? '正在记录键盘活动'
      : '键盘采集未启动';
  return `<div class="capture-chip ${state.captureEnabled ? 'on' : 'off'}"><i></i>${stateText}</div>`;
}

function renderActivity() {
  const data = state.activityData || { activeMs: 0, keyCount: 0, keyCounts: {}, days: [] };
  const title = `${activityRangeLabel(state.activityRange)}键盘热力图`;
  const hasData = (data.keyCount || 0) > 0;
  const keysPerMinute = data.activeMs > 0
    ? Math.round((data.keyCount || 0) / (data.activeMs / 60000))
    : null;
  dataPane.innerHTML = `
    <div class="activity-view">
      <header class="data-head">
        <div class="head-copy">
          <h1>键盘活跃</h1>
          <p>看看专注发生在什么时候。</p>
        </div>
        ${captureChipHtml()}
      </header>
      <div class="range-row">
        <span class="range-label">时间范围</span>
        <div class="segmented">${rangeButtons('activity', state.activityRange)}</div>
        <span class="range-dates">${rangeDateLabel(state.activityRange)}</span>
      </div>
      <div class="metric-grid">
        <div class="metric-card">
          <span>键盘活跃时间</span>
          <strong>${formatDurationMs(data.activeMs || 0)}</strong>
          <small>合并连续输入与按住区间</small>
        </div>
        <div class="metric-card">
          <span>按键次数</span>
          <strong>${(data.keyCount || 0).toLocaleString('zh-CN')}</strong>
          <small>包含修饰键，过滤长按重复</small>
        </div>
        <div class="metric-card">
          <span>输入速度</span>
          <strong>${keysPerMinute == null ? '—' : `${keysPerMinute.toLocaleString('zh-CN')} 键/分钟`}</strong>
          <small>按键盘活跃时间平均</small>
        </div>
      </div>
      <section class="data-card">
        <div class="card-head">
          <h2>${title}</h2>
          <span class="section-meta">${state.activityLoading ? '正在更新…' : rangeDateLabel(state.activityRange)}</span>
        </div>
        ${hasData ? renderKeyboardKeys(data.keyCounts) : '<div class="data-empty">这段时间还没有按键记录。保持 Toccata 运行，输入会自动统计。</div>'}
        ${hasData ? '<div class="keyboard-legend"><span>少</span><i class="heat-swatch level-1"></i><i class="heat-swatch level-2"></i><i class="heat-swatch level-3"></i><i class="heat-swatch level-4"></i><span>多</span></div>' : ''}
      </section>
      <section class="data-card">
        <div class="card-head">
          <h2>活跃时间线</h2>
          <span class="section-meta">每格 15 分钟，颜色越深表示活跃时间越长</span>
        </div>
        ${renderTimeline(data.days)}
      </section>
      <p class="calc-note">
        每次有效按下覆盖后续 5 秒，持续按住的时间也计入；重叠区间只计算一次，未被覆盖的停顿不计入。
        键盘活跃时间是一种使用时间估算，阅读、看视频或仅用鼠标时可能没有记录。
      </p>
    </div>`;
}

function appDisplayName(app) {
  const key = String(app || '').toLowerCase();
  if (APP_NAMES[key]) return APP_NAMES[key];
  const base = key.replace(/\.exe$/i, '').replace(/[-_]/g, ' ').trim();
  if (!base) return '未知应用';
  return base.replace(/\b\w/g, char => char.toUpperCase());
}

function appColor(app) {
  let hash = 0;
  for (const char of String(app || '')) hash = (hash * 31 + char.charCodeAt(0)) % 360;
  return `hsl(${hash}, 62%, 92%)`;
}

function renderAppList(apps) {
  if (!apps || !apps.length) {
    return '<div class="data-empty">这段时间还没有应用使用记录。</div>';
  }
  const max = Math.max(1, ...apps.map(app => app.activeMs || 1));
  return `<div class="app-list">${apps.map(app => {
    const name = appDisplayName(app.app);
    const initial = name.slice(0, 1).toUpperCase();
    const ratio = Math.max(2, Math.min(100, (app.activeMs / max) * 100));
    return `
      <div class="app-row">
        <div class="app-icon" style="background:${appColor(app.app)}">${escapeHtml(initial)}</div>
        <div class="app-main">
          <div class="app-line">
            <span class="app-name" title="${escapeHtml(app.app)}">${escapeHtml(name)}</span>
            <span class="app-duration">${formatDurationMs(app.activeMs)}</span>
            <span class="app-keys">${app.keyCount.toLocaleString('zh-CN')} 次按键</span>
          </div>
          <div class="app-bar"><i style="width:${ratio.toFixed(1)}%"></i></div>
        </div>
      </div>`;
  }).join('')}</div>`;
}

function renderActivityHeatmap(daily) {
  const counts = new Map();
  for (const entry of daily || []) counts.set(entry.day, entry.keyCount);
  const today = startOfDay(new Date());
  const start = addDays(weekStartDate(today), -25 * 7);
  let max = 1;
  for (const value of counts.values()) max = Math.max(max, value || 0);

  let weeks = '';
  for (let week = 0; week < 26; week += 1) {
    let cells = '';
    for (let dayIndex = 0; dayIndex < 7; dayIndex += 1) {
      const date = addDays(start, week * 7 + dayIndex);
      if (date.getTime() > today.getTime()) {
        cells += '<div class="heat-cell future"></div>';
        continue;
      }
      const key = dayKeyFromDate(date);
      const count = counts.get(key) || 0;
      const ratio = count > 0 ? count / max : 0;
      const level = count === 0 ? 0 : ratio <= 0.25 ? 1 : ratio <= 0.5 ? 2 : ratio <= 0.75 ? 3 : 4;
      const isToday = key === todayKey();
      cells += `<div class="heat-cell level-${level}${isToday ? ' today' : ''}" title="${key} · ${count} 次按键"></div>`;
    }
    weeks += `<div class="heat-week">${cells}</div>`;
  }

  const total = [...counts.values()].reduce((sum, value) => sum + value, 0);
  return `
    <div class="heatmap-wrap">
      <div class="heatmap-weeks">${weeks}</div>
      <div class="heatmap-foot">
        <span>最近 26 周 · 共 ${total.toLocaleString('zh-CN')} 次按键</span>
        <span class="heatmap-legend">少 <i class="heat-swatch level-1"></i><i class="heat-swatch level-2"></i><i class="heat-swatch level-3"></i><i class="heat-swatch level-4"></i> 多</span>
      </div>
    </div>`;
}

function renderRanking() {
  const data = state.rankingData || { apps: [], activeMs: 0, keyCount: 0 };
  const days = dayRangesForRange(state.rankingRange);
  dataPane.innerHTML = `
    <div class="activity-view">
      <header class="data-head">
        <div class="head-copy">
          <h1>应用与趋势</h1>
          <p>回看键盘活动与应用使用趋势。</p>
        </div>
        ${captureChipHtml()}
      </header>
      <section class="data-card">
        <div class="card-head">
          <h2>活跃日历</h2>
          <span class="section-meta">颜色越深，按键次数越多</span>
        </div>
        ${renderActivityHeatmap(state.rankingDaily)}
      </section>
      <section class="data-card">
        <div class="card-head">
          <h2>应用活跃度</h2>
          <span class="section-meta">按键盘活动估算使用时间</span>
        </div>
        <div class="range-row compact">
          <div class="segmented">${rangeButtons('ranking', state.rankingRange)}</div>
          <span class="range-dates">${days.length ? formatLongDate(days[0].startMs / 1000) : ''}${days.length > 1 ? ' 起' : ''}</span>
          <span class="section-meta">${formatDurationMs(data.activeMs || 0)} · ${(data.keyCount || 0).toLocaleString('zh-CN')} 次按键</span>
        </div>
        ${state.rankingLoading ? '<div class="data-empty">正在读取应用使用记录…</div>' : renderAppList(data.apps)}
      </section>
      <p class="calc-note">
        应用排行根据按键事件时所在的前台应用估算；只记录进程名、时间与键码，不保存输入内容、窗口标题或屏幕截图。
        全屏程序、管理员权限窗口可能无法被普通权限进程捕获。
      </p>
    </div>`;
}

async function loadActivity() {
  if (!invoke || state.activityLoading) return;
  state.activityLoading = true;
  if (state.view === 'activity') renderActivity();
  try {
    const days = dayRangesForRange(state.activityRange);
    const [data, capture] = await Promise.all([
      invoke('keyboard_snapshot', { days, nowMs: Date.now() }),
      invoke('capture_status').catch(() => false)
    ]);
    state.activityData = data;
    state.captureEnabled = Boolean(capture);
  } catch (error) {
    if (state.view === 'activity') toast(`读取键盘活动失败：${error}`);
  } finally {
    state.activityLoading = false;
    if (state.view === 'activity') renderActivity();
  }
}

async function loadRanking() {
  if (!invoke || state.rankingLoading) return;
  state.rankingLoading = true;
  if (state.view === 'ranking') renderRanking();
  try {
    const heatStart = addDays(weekStartDate(new Date()), -25 * 7);
    const days = dayRangesForRange(state.rankingRange);
    const [data, daily, capture] = await Promise.all([
      invoke('keyboard_snapshot', { days, nowMs: Date.now() }),
      invoke('keyboard_daily', { startMs: heatStart.getTime(), endMs: Date.now() }),
      invoke('capture_status').catch(() => false)
    ]);
    state.rankingData = data;
    state.rankingDaily = daily;
    state.captureEnabled = Boolean(capture);
  } catch (error) {
    if (state.view === 'ranking') toast(`读取应用趋势失败：${error}`);
  } finally {
    state.rankingLoading = false;
    if (state.view === 'ranking') renderRanking();
  }
}

/* ---------- render: all ---------- */

function renderAll() {
  if (state.selectedId && !state.byId.has(state.selectedId)) state.selectedId = null;
  renderSidebar();

  const isTaskView = TASK_VIEWS.includes(state.view);
  const isDataView = ['activity', 'ranking'].includes(state.view);
  tasksArea.classList.toggle('hidden', !isTaskView);
  calendarPane.classList.toggle('hidden', state.view !== 'calendar');
  dataPane.classList.toggle('hidden', !isDataView);

  if (isTaskView) {
    renderHeader();
    renderList();
    renderDetail();
  } else if (state.view === 'calendar') {
    renderCalendar();
  } else if (state.view === 'activity') {
    renderActivity();
  } else if (state.view === 'ranking') {
    renderRanking();
  }
}

async function refresh() {
  try {
    state.tasks = await invoke('list_tasks');
    indexTasks();
    renderAll();
    checkReminders();
  } catch (error) {
    toast(`读取任务失败：${error}`);
  }
}

/* ---------- navigation ---------- */

function setView(view) {
  if (view === state.view) return;
  flushSave();
  state.view = view;
  state.selectedId = null;
  state.query = '';
  state.collapsed.clear();
  state.showCompleted = true;
  if (view === 'calendar') {
    state.calendarMonth = startOfMonth(new Date());
    state.calendarDay = startOfDay(new Date());
  }
  renderAll();
  if (view === 'activity') {
    state.activityData = null;
    loadActivity();
  } else if (view === 'ranking') {
    state.rankingData = null;
    state.rankingDaily = null;
    loadRanking();
  }
}

function setPeriod(period) {
  if (period === state.period) return;
  flushSave();
  state.period = period;
  state.selectedId = null;
  renderAll();
}

function selectTask(id) {
  if (state.selectedId !== id) flushSave();
  state.selectedId = id;
  fillEditor(true);
  renderList();
  if (state.view === 'calendar') renderCalendar();
}

function closeDetail() {
  flushSave();
  state.selectedId = null;
  renderList();
  updateDetailChrome();
}

function openTaskFromCalendar(id) {
  flushSave();
  state.view = 'inbox';
  state.query = '';
  state.selectedId = id;
  fillEditor(true);
  renderAll();
}

/* ---------- saving ---------- */

function currentEditorValues() {
  const dateValue = dateEl.value;
  const timeValue = timeEl.value;
  let plannedAt = null;
  let plannedHasTime = false;
  if (dateValue) {
    const [y, m, d] = dateValue.split('-').map(Number);
    const [hour, minute, second] = timeValue ? timeValue.split(':').map(Number) : [0, 0, 0];
    if (y && m && d) {
      plannedAt = new Date(y, m - 1, d, hour || 0, minute || 0, second || 0, 0).getTime() / 1000;
      plannedHasTime = Boolean(timeValue);
    }
  }

  const lead = Number(document.getElementById('reminder-lead').value);
  return {
    title: titleEl.value.trim(),
    details: detailsEl.value,
    plannedAt,
    plannedHasTime,
    reminderEnabled: document.getElementById('reminder-toggle').checked,
    reminderLeadMinutes: Number.isFinite(lead)
      ? Math.min(10080, Math.max(0, Math.round(lead)))
      : state.reminderSettings.leadMinutes
  };
}

function scheduleSave() {
  state.dirty = true;
  state.saveState = 'saving';
  updateSaveState();
  clearTimeout(state.saveTimer);
  state.saveTimer = setTimeout(() => saveEditor({ silent: true }), 550);
}

async function saveEditor(options = {}) {
  const task = selectedTask();
  if (!task) return;
  const taskId = task.id;
  const values = currentEditorValues();

  if (!values.title) {
    state.saveState = 'invalid';
    state.dirty = true;
    updateSaveState();
    if (!options.silent) toast('任务名称不能为空');
    return;
  }

  const snapshot = state.snapshot || {};
  const scheduleChanged = values.plannedAt !== snapshot.plannedAt
    || values.plannedHasTime !== snapshot.plannedHasTime
    || values.reminderEnabled !== snapshot.reminderEnabled
    || values.reminderLeadMinutes !== snapshot.reminderLeadMinutes;
  const unchanged = values.title === snapshot.title
    && values.details === (snapshot.details || '')
    && !scheduleChanged;
  if (unchanged) {
    state.dirty = false;
    state.saveState = 'saved';
    updateSaveState();
    return;
  }

  state.saveState = 'saving';
  updateSaveState();
  try {
    await invoke('update_task', {
      id: taskId,
      title: values.title,
      details: values.details,
      plannedAt: values.plannedAt,
      plannedHasTime: values.plannedHasTime,
      reminderEnabled: values.reminderEnabled,
      reminderLeadMinutes: values.reminderLeadMinutes
    });
    if (state.selectedId === taskId) {
      state.snapshot = values;
      state.dirty = false;
      state.saveState = 'saved';
      if (scheduleChanged) state.firedReminders.delete(taskId);
    }
    await refresh();
    if (state.selectedId === taskId) updateDetailChrome();
  } catch (error) {
    if (state.selectedId === taskId) {
      state.saveState = 'error';
      updateSaveState();
    }
    toast(`保存失败：${error}`);
  }
}

function flushSave() {
  clearTimeout(state.saveTimer);
  if (state.dirty) return saveEditor({ silent: true });
  return Promise.resolve();
}

/* ---------- schedule editor ---------- */

function openDatePopover() {
  const task = selectedTask();
  if (!task) return;
  dateEl.value = task.plannedAt != null ? dayKeyFromSeconds(task.plannedAt) : '';
  timeEl.value = task.plannedAt != null && task.plannedHasTime ? timeInputValue(task.plannedAt) : '';
  document.getElementById('date-popover').classList.remove('hidden');
}

function closeDatePopover() {
  document.getElementById('date-popover').classList.add('hidden');
}

function applyDatePopover() {
  if (!dateEl.value && timeEl.value) dateEl.value = todayKey();
  closeDatePopover();
  state.dirty = true;
  saveEditor();
}

/* ---------- global reminder settings ---------- */

function switchSettingsTab(tab) {
  document.querySelectorAll('.settings-tab').forEach(button => {
    button.classList.toggle('active', button.dataset.settingsTab === tab);
  });
  document.getElementById('settings-reminder').classList.toggle('hidden', tab !== 'reminder');
  document.getElementById('settings-window').classList.toggle('hidden', tab !== 'window');
}

function openSettings(tab = 'reminder') {
  const settings = state.reminderSettings;
  document.getElementById('setting-default-enabled').checked = settings.defaultEnabled;
  document.getElementById('setting-lead').value = String(settings.leadMinutes);
  document.getElementById('setting-time').value = settings.dateOnlyTime;
  document.getElementById('setting-fullscreen').checked = settings.silentFullscreen;

  const mode = ['minimize', 'exit', 'ask'].includes(state.closeMode) ? state.closeMode : 'ask';
  const radio = document.querySelector(`input[name="close-mode"][value="${mode}"]`);
  if (radio) radio.checked = true;

  switchSettingsTab(tab);
  document.getElementById('settings-modal').classList.remove('hidden');
}

function closeSettings() {
  const lead = Number(document.getElementById('setting-lead').value);
  const time = document.getElementById('setting-time').value;
  state.reminderSettings = {
    defaultEnabled: document.getElementById('setting-default-enabled').checked,
    leadMinutes: Number.isFinite(lead) ? Math.min(10080, Math.max(0, Math.round(lead))) : DEFAULT_REMINDER_SETTINGS.leadMinutes,
    silentFullscreen: document.getElementById('setting-fullscreen').checked,
    dateOnlyTime: /^\d{2}:\d{2}$/.test(time) ? time : DEFAULT_REMINDER_SETTINGS.dateOnlyTime
  };
  writeSetting('toccata.reminderSettings', JSON.stringify(state.reminderSettings));

  const mode = document.querySelector('input[name="close-mode"]:checked')?.value;
  if (['minimize', 'exit', 'ask'].includes(mode)) {
    state.closeMode = mode;
    writeSetting('toccata.closeBehavior', mode);
  }

  document.getElementById('settings-modal').classList.add('hidden');
  updateDetailChrome();
  toast('设置已保存');
}

function handleCloseRequest() {
  if (state.closePromptOpen) return;
  if (state.closeMode === 'minimize') {
    invoke('minimize_main_window').catch(error => toast(`最小化失败：${error}`));
    return;
  }
  if (state.closeMode === 'exit') {
    invoke('quit_app');
    return;
  }

  state.closePromptOpen = true;
  const minimize = document.querySelector('input[name="close-choice"][value="minimize"]');
  if (minimize) minimize.checked = true;
  document.getElementById('close-remember').checked = true;
  document.getElementById('close-modal').classList.remove('hidden');
}

function confirmCloseChoice() {
  const selected = document.querySelector('input[name="close-choice"]:checked')?.value || 'minimize';
  const remember = document.getElementById('close-remember').checked;
  if (remember) {
    state.closeMode = selected;
    writeSetting('toccata.closeBehavior', selected);
  }
  document.getElementById('close-modal').classList.add('hidden');
  state.closePromptOpen = false;

  if (selected === 'exit') invoke('quit_app');
  else invoke('minimize_main_window').catch(error => toast(`最小化失败：${error}`));
}

function cancelCloseChoice() {
  document.getElementById('close-modal').classList.add('hidden');
  state.closePromptOpen = false;
}

/* ---------- reminders ---------- */

function enqueueReminder(task) {
  state.reminderQueue.push(task);
  if (!state.reminderShowing) showNextReminder();
}

function showNextReminder() {
  const task = state.reminderQueue.shift();
  if (!task) {
    state.reminderShowing = false;
    state.reminderTaskId = null;
    return;
  }
  state.reminderShowing = true;
  state.reminderTaskId = task.id;
  document.getElementById('reminder-title').textContent = task.title;
  const lead = task.reminderLeadMinutes == null ? state.reminderSettings.leadMinutes : task.reminderLeadMinutes;
  document.getElementById('reminder-when').textContent =
    `计划时间 ${formatFull(task.plannedAt)} · 提前 ${lead} 分钟提醒`;
  document.getElementById('reminder-modal').classList.remove('hidden');

  if (invoke) {
    invoke('show_notification', { title: 'Toccata 提醒', body: task.title }).catch(() => {});
  }
}

function dismissReminder() {
  document.getElementById('reminder-modal').classList.add('hidden');
  state.reminderShowing = false;
  state.reminderTaskId = null;
  showNextReminder();
}

function openReminderTask() {
  const id = state.reminderTaskId;
  document.getElementById('reminder-modal').classList.add('hidden');
  state.reminderShowing = false;
  state.reminderTaskId = null;
  if (id) {
    const task = state.byId.get(id);
    if (task) {
      if (!TASK_VIEWS.includes(state.view)) setView('inbox');
      state.selectedId = id;
      fillEditor(true);
      renderAll();
    }
  }
  showNextReminder();
}

async function checkReminders() {
  if (!invoke || !state.tasks.length) return;
  const now = nowSeconds();
  for (const task of state.tasks) {
    if (!task.reminderEnabled || task.plannedAt == null || task.completedAt || task.notifiedAt != null) {
      continue;
    }
    const due = reminderTimeFor(task);
    if (due == null || due > now || now - due > 86400) continue;
    if (state.firedReminders.has(task.id)) continue;
    state.firedReminders.add(task.id);

    if (state.reminderSettings.silentFullscreen) {
      try {
        const fullscreen = await invoke('is_fullscreen_active');
        if (fullscreen) {
          state.firedReminders.delete(task.id);
          continue;
        }
      } catch {
        /* If detection fails, keep the reminder. */
      }
    }

    enqueueReminder(task);
    try {
      await invoke('mark_reminder_notified', { id: task.id, at: now });
      task.notifiedAt = now;
    } catch {
      /* Retry on the next poll if the write failed. */
      state.firedReminders.delete(task.id);
    }
  }
}

/* ---------- mutations ---------- */

async function createTask(title, options = {}) {
  const text = title.trim();
  if (!text) return null;
  const plannedAt = options.plannedAt ?? null;
  const hasTime = Boolean(options.hasTime);
  try {
    const created = await invoke('create_task', {
      parentId: null,
      title: text,
      details: '',
      plannedAt,
      plannedHasTime: hasTime,
      reminderEnabled: Boolean(options.reminderEnabled),
      reminderLeadMinutes: options.reminderLeadMinutes ?? state.reminderSettings.leadMinutes
    });
    await refresh();
    if (created && created.id) {
      state.selectedId = created.id;
      fillEditor(true);
      renderList();
      if (plannedAt != null) {
        toast(`已安排 ${hasTime ? formatFull(plannedAt) : formatLongDate(plannedAt)}`);
      }
    }
    return created;
  } catch (error) {
    toast(`创建失败：${error}`);
    return null;
  }
}

async function createSubtask(title) {
  const text = title.trim();
  const parent = selectedTask();
  if (!text || !parent || parent.parentId) return;
  try {
    await flushSave();
    await invoke('create_task', {
      parentId: parent.id,
      title: text,
      details: '',
      plannedAt: null,
      plannedHasTime: false,
      reminderEnabled: false,
      reminderLeadMinutes: null
    });
    await refresh();
    document.getElementById('new-subtask').focus();
  } catch (error) {
    toast(`创建失败：${error}`);
  }
}

async function toggleComplete(id) {
  const task = state.byId.get(id);
  if (!task) return;
  await flushSave();
  try {
    await invoke('set_task_completed', { id, completed: !task.completedAt });
    await refresh();
  } catch (error) {
    toast(`操作失败：${error}`);
  }
}

async function togglePin(id) {
  const task = state.byId.get(id);
  if (!task) return;
  const root = rootOf(task);
  try {
    await invoke('set_task_pinned', { id: root.id, pinned: !root.isPinned });
    await refresh();
  } catch (error) {
    toast(`操作失败：${error}`);
  }
}

let confirmResolver = null;

function askConfirm(message) {
  document.getElementById('confirm-text').textContent = message;
  document.getElementById('confirm').classList.remove('hidden');
  return new Promise(resolve => {
    confirmResolver = resolve;
  });
}

function settleConfirm(value) {
  document.getElementById('confirm').classList.add('hidden');
  const resolve = confirmResolver;
  confirmResolver = null;
  if (resolve) resolve(value);
}

async function deleteTask(id) {
  const task = state.byId.get(id);
  if (!task) return;
  const children = childrenOf(id);
  const message = children.length
    ? '任务、所有子任务及其详情都会删除。此操作无法撤销。'
    : '任务详情也会删除。此操作无法撤销。';
  if (!await askConfirm(message)) return;

  try {
    await invoke('delete_task', { id });
    if (state.selectedId === id) {
      state.selectedId = null;
      state.dirty = false;
    }
    await refresh();
  } catch (error) {
    toast(`删除失败：${error}`);
  }
}

function toggleCollapse(id) {
  if (state.collapsed.has(id)) state.collapsed.delete(id);
  else state.collapsed.add(id);
  renderList();
}

/* ---------- markdown ---------- */

function escapeHtml(value) {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function markdownInline(text) {
  let html = escapeHtml(text);
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
  html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/(^|[^*])\*([^*]+)\*/g, '$1<em>$2</em>');
  html = html.replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g,
    '<a href="$2" target="_blank" rel="noreferrer">$1</a>');
  return html;
}

function markdownToHtml(text) {
  const lines = String(text || '').replace(/\r\n/g, '\n').split('\n');
  const out = [];
  let inCode = false;
  let codeLines = [];
  let listType = null;

  const closeList = () => {
    if (listType) {
      out.push(`</${listType}>`);
      listType = null;
    }
  };

  for (const line of lines) {
    if (line.trim().startsWith('```')) {
      if (inCode) {
        out.push(`<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`);
        codeLines = [];
        inCode = false;
      } else {
        closeList();
        inCode = true;
      }
      continue;
    }
    if (inCode) {
      codeLines.push(line);
      continue;
    }

    const heading = line.match(/^(#{1,3})\s+(.*)$/);
    if (heading) {
      closeList();
      const level = heading[1].length;
      out.push(`<h${level}>${markdownInline(heading[2])}</h${level}>`);
      continue;
    }

    const quote = line.match(/^>\s?(.*)$/);
    if (quote) {
      closeList();
      out.push(`<blockquote>${markdownInline(quote[1])}</blockquote>`);
      continue;
    }

    const unordered = line.match(/^\s*[-*]\s+(.*)$/);
    if (unordered) {
      if (listType !== 'ul') {
        closeList();
        out.push('<ul>');
        listType = 'ul';
      }
      out.push(`<li>${markdownInline(unordered[1])}</li>`);
      continue;
    }

    const ordered = line.match(/^\s*\d+\.\s+(.*)$/);
    if (ordered) {
      if (listType !== 'ol') {
        closeList();
        out.push('<ol>');
        listType = 'ol';
      }
      out.push(`<li>${markdownInline(ordered[1])}</li>`);
      continue;
    }

    if (!line.trim()) {
      closeList();
      continue;
    }

    closeList();
    out.push(`<p>${markdownInline(line)}</p>`);
  }

  if (inCode) out.push(`<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`);
  closeList();
  return out.join('');
}

/* ---------- review copy ---------- */

function buildReviewText() {
  const rangeLabel = state.period === 'week' ? '本周' : '最近 7 天';
  const completed = state.tasks
    .filter(task => completedInPeriod(task, state.period))
    .sort((a, b) => b.completedAt - a.completedAt);
  const pending = state.tasks
    .filter(task => !task.completedAt)
    .sort((a, b) => a.title.localeCompare(b.title, 'zh-Hans-CN'));

  const lines = [`# 任务回顾（${rangeLabel}）`, ''];
  lines.push('## 已完成', '');
  if (!completed.length) lines.push('（暂无完成记录）');
  for (const task of completed) {
    const parent = task.parentId ? state.byId.get(task.parentId) : null;
    const suffix = parent ? `（父任务：${parent.title}）` : '';
    lines.push(`- [x] ${task.title}${suffix} — ${formatFull(task.completedAt)}`);
    if (task.details && task.details.trim()) {
      lines.push(`  ${task.details.trim().replace(/\n/g, '\n  ')}`);
    }
  }
  lines.push('', '## 待推进', '');
  if (!pending.length) lines.push('（暂无未完成任务）');
  for (const task of pending) {
    const parent = task.parentId ? state.byId.get(task.parentId) : null;
    const suffix = parent ? `（父任务：${parent.title}）` : '';
    lines.push(`- [ ] ${task.title}${suffix}`);
  }
  return lines.join('\n');
}

async function copyReview() {
  const text = buildReviewText();
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const area = document.createElement('textarea');
    area.value = text;
    area.style.position = 'fixed';
    area.style.opacity = '0';
    document.body.appendChild(area);
    area.select();
    document.execCommand('copy');
    area.remove();
  }
  toast('回顾已复制，可以粘贴到周报里。');
}

/* ---------- toast ---------- */

let toastTimer = null;

function toast(message) {
  const el = document.getElementById('toast');
  el.textContent = message;
  el.classList.remove('hidden');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.add('hidden'), 2600);
}

/* ---------- events ---------- */

function bindEvents() {
  document.querySelector('.nav').addEventListener('click', event => {
    const button = event.target.closest('[data-view]');
    if (button) setView(button.dataset.view);
  });

  document.getElementById('period-control').addEventListener('click', event => {
    const button = event.target.closest('[data-period]');
    if (button) setPeriod(button.dataset.period);
  });

  document.getElementById('add-btn').addEventListener('click', submitComposer);
  document.getElementById('add-enter').addEventListener('click', submitComposer);
  document.getElementById('new-task').addEventListener('keydown', event => {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    submitComposer();
  });

  document.getElementById('search').addEventListener('input', event => {
    state.query = event.currentTarget.value;
    document.getElementById('clear-search').classList.toggle('hidden', !state.query);
    renderList();
  });

  document.getElementById('clear-search').addEventListener('click', () => {
    state.query = '';
    searchEl.value = '';
    document.getElementById('clear-search').classList.add('hidden');
    renderList();
    searchEl.focus();
  });

  document.getElementById('display-toggle').addEventListener('click', () => {
    state.display = state.display === 'details' ? 'titles' : 'details';
    writeSetting('toccata.listDisplay', state.display);
    document.getElementById('display-label').textContent = state.display === 'details' ? '含详情' : '仅标题';
    renderList();
  });

  document.getElementById('sort-select').addEventListener('change', event => {
    state.sort = event.currentTarget.value;
    writeSetting('toccata.sortOrder', state.sort);
    renderList();
  });

  listEl.addEventListener('click', event => {
    const section = event.target.closest('[data-section="completed"]');
    if (section) {
      state.showCompleted = !state.showCompleted;
      renderList();
      return;
    }
    const wrap = event.target.closest('.row-wrap');
    if (!wrap) return;
    const id = wrap.dataset.id;
    if (event.target.closest('.row-check')) {
      toggleComplete(id);
      return;
    }
    if (event.target.closest('.row-expand')) {
      toggleCollapse(id);
      return;
    }
    selectTask(id);
  });

  document.getElementById('copy-review').addEventListener('click', copyReview);

  titleEl.addEventListener('input', () => {
    scheduleSave();
    autoGrowTitle();
  });
  detailsEl.addEventListener('focus', () => {
    if (!state.detailsEditing) {
      state.detailsEditing = true;
      renderDetailsMode();
    }
  });
  detailsEl.addEventListener('input', () => {
    state.detailsEditing = true;
    scheduleSave();
    renderDetailsMode();
  });

  document.getElementById('preview-btn').addEventListener('click', () => {
    const hasText = detailsEl.value.trim().length > 0;
    if (state.detailsEditing || !hasText) {
      state.detailsEditing = false;
      renderDetailsMode();
    } else {
      state.detailsEditing = true;
      renderDetailsMode();
      detailsEl.focus();
    }
  });

  previewEl.addEventListener('click', () => {
    state.detailsEditing = true;
    renderDetailsMode();
    detailsEl.focus();
  });

  document.getElementById('date-card').addEventListener('click', () => {
    if (!selectedTask()) return;
    const popover = document.getElementById('date-popover');
    if (popover.classList.contains('hidden')) openDatePopover();
    else closeDatePopover();
  });

  document.getElementById('date-card').addEventListener('keydown', event => {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    document.getElementById('date-card').click();
  });

  document.getElementById('date-apply').addEventListener('click', applyDatePopover);
  document.getElementById('date-clear').addEventListener('click', () => {
    dateEl.value = '';
    timeEl.value = '';
    closeDatePopover();
    state.dirty = true;
    saveEditor();
  });

  document.getElementById('date-popover').addEventListener('keydown', event => {
    if (event.key === 'Enter') {
      event.preventDefault();
      applyDatePopover();
    } else if (event.key === 'Escape') {
      closeDatePopover();
    }
  });

  document.addEventListener('click', event => {
    const popover = document.getElementById('date-popover');
    if (popover.classList.contains('hidden')) return;
    if (!event.target.closest('.date-section')) closeDatePopover();
  });

  document.getElementById('reminder-toggle').addEventListener('change', event => {
    const task = selectedTask();
    if (!task) return;
    if (event.currentTarget.checked) {
      if (task.plannedAt == null) {
        event.currentTarget.checked = false;
        toast('请先设置计划日期');
        return;
      }
      if (!task.plannedHasTime) {
        dateEl.value = dayKeyFromSeconds(task.plannedAt);
        timeEl.value = state.reminderSettings.dateOnlyTime;
        toast(`日期任务将在 ${state.reminderSettings.dateOnlyTime} 提醒，可在计划时间中调整`);
      }
    }
    saveEditor();
  });

  document.getElementById('reminder-lead').addEventListener('change', () => {
    const input = document.getElementById('reminder-lead');
    const value = Number(input.value);
    input.value = String(Number.isFinite(value) ? Math.min(10080, Math.max(0, Math.round(value))) : state.reminderSettings.leadMinutes);
    if (selectedTask()) saveEditor();
  });

  document.getElementById('reminder-defaults').addEventListener('click', () => openSettings('reminder'));
  document.getElementById('open-settings').addEventListener('click', () => openSettings('reminder'));
  document.getElementById('settings-close').addEventListener('click', closeSettings);
  document.querySelectorAll('.settings-tab').forEach(button => {
    button.addEventListener('click', () => switchSettingsTab(button.dataset.settingsTab));
  });
  document.getElementById('close-cancel').addEventListener('click', cancelCloseChoice);
  document.getElementById('close-confirm').addEventListener('click', confirmCloseChoice);
  document.getElementById('close-modal').addEventListener('click', event => {
    if (event.target.id === 'close-modal') cancelCloseChoice();
  });
  document.getElementById('settings-modal').addEventListener('click', event => {
    if (event.target.id === 'settings-modal') closeSettings();
  });

  document.getElementById('reminder-dismiss').addEventListener('click', dismissReminder);
  document.getElementById('reminder-open').addEventListener('click', openReminderTask);

  document.getElementById('new-subtask').addEventListener('keydown', event => {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    const input = event.currentTarget;
    const title = input.value;
    input.value = '';
    createSubtask(title);
  });

  subtaskListEl.addEventListener('click', event => {
    const row = event.target.closest('.subtask-row');
    if (!row) return;
    if (event.target.closest('.row-check')) {
      toggleComplete(row.dataset.id);
      return;
    }
    selectTask(row.dataset.id);
  });

  document.getElementById('complete-btn').addEventListener('click', () => {
    if (state.selectedId) toggleComplete(state.selectedId);
  });

  document.getElementById('pin-btn').addEventListener('click', () => {
    if (state.selectedId) togglePin(state.selectedId);
  });

  document.getElementById('save-btn').addEventListener('click', () => saveEditor());
  document.getElementById('delete-btn').addEventListener('click', () => {
    if (state.selectedId) deleteTask(state.selectedId);
  });

  document.getElementById('back-parent').addEventListener('click', () => {
    const task = selectedTask();
    if (task && task.parentId) selectTask(task.parentId);
  });

  dataPane.addEventListener('click', event => {
    const activity = event.target.closest('[data-activity-range]');
    if (activity) {
      state.activityRange = activity.dataset.activityRange;
      state.activityData = null;
      loadActivity();
      return;
    }
    const ranking = event.target.closest('[data-ranking-range]');
    if (ranking) {
      state.rankingRange = ranking.dataset.rankingRange;
      state.rankingData = null;
      loadRanking();
    }
  });

  document.getElementById('cal-prev').addEventListener('click', () => {
    state.calendarMonth = addMonths(state.calendarMonth, -1);
    state.calendarDay = startOfMonth(state.calendarMonth);
    renderCalendar();
  });
  document.getElementById('cal-next').addEventListener('click', () => {
    state.calendarMonth = addMonths(state.calendarMonth, 1);
    state.calendarDay = startOfMonth(state.calendarMonth);
    renderCalendar();
  });
  document.getElementById('cal-today').addEventListener('click', () => {
    state.calendarMonth = startOfMonth(new Date());
    state.calendarDay = startOfDay(new Date());
    renderCalendar();
  });
  document.getElementById('calendar-grid').addEventListener('click', event => {
    const cell = event.target.closest('[data-date]');
    if (!cell) return;
    const [y, m, d] = cell.dataset.date.split('-').map(Number);
    state.calendarDay = new Date(y, m - 1, d, 0, 0, 0, 0);
    state.calendarMonth = startOfMonth(state.calendarDay);
    renderCalendar();
  });
  document.getElementById('day-tasks').addEventListener('click', event => {
    const card = event.target.closest('[data-id]');
    if (card) openTaskFromCalendar(card.dataset.id);
  });

  document.getElementById('confirm-cancel').addEventListener('click', () => settleConfirm(false));
  document.getElementById('confirm-ok').addEventListener('click', () => settleConfirm(true));
  document.getElementById('confirm').addEventListener('click', event => {
    if (event.target.id === 'confirm') settleConfirm(false);
  });

  document.addEventListener('keydown', event => {
    if (event.key === 'Escape' && !document.getElementById('confirm').classList.contains('hidden')) {
      settleConfirm(false);
      return;
    }
    if (event.key === 'Escape' && !document.getElementById('close-modal').classList.contains('hidden')) {
      cancelCloseChoice();
      return;
    }
    if (event.key === 'Escape' && !document.getElementById('settings-modal').classList.contains('hidden')) {
      closeSettings();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
      event.preventDefault();
      saveEditor();
    }
  });

  window.addEventListener('beforeunload', () => {
    flushSave();
  });
}

function submitComposer() {
  const input = document.getElementById('new-task');
  const raw = input.value;
  input.value = '';
  if (!raw.trim()) return;
  const parsed = parseTaskInput(raw);
  createTask(parsed.title, {
    plannedAt: parsed.date ? parsed.date.getTime() / 1000 : null,
    hasTime: parsed.hasTime,
    reminderEnabled: state.reminderSettings.defaultEnabled,
    reminderLeadMinutes: state.reminderSettings.leadMinutes
  });
}

/* ---------- boot ---------- */

async function boot() {
  bindEvents();

  if (window.__TAURI__ && window.__TAURI__.event) {
    window.__TAURI__.event.listen('close-requested', () => handleCloseRequest());
  }

  if (!invoke) {
    emptyEl.classList.remove('hidden');
    document.getElementById('empty-title').textContent = '请通过项目根目录的 run.bat 启动 Toccata';
    document.getElementById('empty-text').textContent = '直接打开这个 HTML 文件时不会连接任务数据库。';
    return;
  }

  if (!['manual', 'created-desc', 'created-asc', 'title', 'planned'].includes(state.sort)) state.sort = 'manual';
  if (!['titles', 'details'].includes(state.display)) state.display = 'titles';

  await refresh();
  setInterval(checkReminders, 20000);
  setInterval(() => {
    if (document.hidden) return;
    if (TASK_VIEWS.includes(state.view)) updateRemainingBadges();
  }, 1000);
  setInterval(() => {
    if (document.hidden) return;
    if (state.view === 'activity') loadActivity();
    else if (state.view === 'ranking') loadRanking();
  }, 5000);
}

boot();
