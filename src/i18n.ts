// Every word the card says, in each language it says them in.
//
// The catalogue is an object of plain values and small functions rather than a
// lookup by string key: a missing entry or a wrong argument is then a type
// error rather than a blank space on the card at runtime.

import type { Meter, SectionKey } from "./types";

export type Lang = "en" | "ko";

export interface Strings {
  /**
   * The language this catalogue is written in, so that dates and times are
   * written in it too: a Korean card should not be dated `Sep 18`.
   */
  lang: Lang;

  /** Title-bar buttons. */
  settings: string;
  refresh: string;
  close: string;

  meterSession: string;
  meterWeeklyAll: string;
  meterWeeklyScoped: (model: string) => string;
  meterWeeklyScopedUnknown: string;
  /** Any limit kind added after this was written, named as the API named it. */
  meterOther: (kind: string, model: string | null) => string;
  resets: (time: string) => string;
  resetsOn: (date: string, time: string) => string;
  paceAhead: (used: number, elapsed: number) => string;
  paceWithin: (used: number, elapsed: number) => string;

  accountPickerTitle: string;
  cachedSuffix: string;

  updated: string;
  cached: string;
  runUsage: string;
  reasons: Record<string, string>;
  extraOn: string;
  extraShare: (percent: number) => string;
  extraOff: string;
  extraOffBecause: (reason: string) => string;

  noCache: string;
  nothingSelected: string;
  readError: (message: string) => string;

  loginMissing: string;
  loginWaitingNote: string;
  loginButton: string;
  loginWaitingButton: string;

  /** Shown in place of the sign-in prompt when there is no CLI to sign in with. */
  claudeMissing: string;
  installButton: string;
  loginFailed: (message: string) => string;

  /** The settings-panel section that lists accounts and adds more. */
  accounts: string;
  accountsTitle: string;
  accountSource: (source: string) => string;
  accountAdded: string;
  addAccount: string;
  addAccountWaiting: string;
  addAccountNote: string;
  addAccountFailed: (message: string) => string;
  removeAccount: string;
  accountsFull: string;

  show: string;
  section: (key: SectionKey) => string;
  refreshEvery: string;
  refreshTitle: string;
  refreshChoice: (seconds: number) => string;
  clock: string;
  clockChoice: (value: string) => string;
  timeFormat: string;
  timeFormatTitle: string;
  timeFormatChoice: (value: string) => string;
  corners: string;
  cornersTitle: string;
  cornerChoice: (px: number) => string;
  language: string;
  languageChoice: (value: string) => string;
  update: string;
  updateCheck: string;
  updateChecking: string;
  updateCurrent: string;
  updateAvailable: (version: string) => string;
  updateInstalling: string;
  updateFailed: string;
}

const en: Strings = {
  lang: "en",

  settings: "Settings",
  refresh: "Refresh",
  close: "Close",

  meterSession: "Current session",
  meterWeeklyAll: "Current week (all models)",
  meterWeeklyScoped: (model) => `Current week (${model})`,
  meterWeeklyScopedUnknown: "Current week (scoped)",
  meterOther: (kind, model) => (model ? `${kind} (${model})` : kind),
  resets: (time) => `Resets ${time}`,
  resetsOn: (date, time) => `Resets ${date}, ${time}`,
  paceAhead: (used, elapsed) =>
    `using faster than the clock (${used}% used, ${elapsed}% elapsed)`,
  paceWithin: (used, elapsed) => `within pace (${used}% used, ${elapsed}% elapsed)`,

  accountPickerTitle: "Quota is per account; these installs are signed in as different ones",
  cachedSuffix: " (cached)",

  updated: "Updated",
  cached: "Cached",
  runUsage: "run /usage to refresh",
  reasons: {
    expired: "sign-in expired",
    rateLimited: "API is rate-limiting",
    requestFailed: "API did not answer",
    noCredentials: "not signed in here",
  },
  extraOn: "extra usage on",
  extraShare: (percent) => `extra usage ${percent}%`,
  extraOff: "extra usage off",
  extraOffBecause: (reason) => `extra usage off (${reason})`,

  noCache: "No cached usage found. Run Claude Code once to populate it.",
  nothingSelected: "Nothing selected. Use the sliders to turn a meter back on.",
  readError: (message) => `Could not read usage: ${message}`,

  loginMissing: "No Claude Code login found on this machine.",
  loginWaitingNote: "Complete the sign-in in the console window.",
  loginButton: "Login Required",
  loginWaitingButton: "Waiting for sign-in…",
  claudeMissing: "Claude Code is not installed on this machine.",
  installButton: "How to Install",
  loginFailed: (message) => `Could not start the sign-in: ${message}`,
  accounts: "Accounts",
  accountsTitle:
    "Quota is per account. Installs found on this machine are listed too, and cannot be removed here.",
  accountSource: (source) => `from ${source}`,
  accountAdded: "added here",
  addAccount: "Add an account",
  addAccountWaiting: "Waiting for sign-in…",
  addAccountNote: "Complete the sign-in in the console window.",
  addAccountFailed: (message) => `Could not add the account: ${message}`,
  removeAccount: "Remove",
  accountsFull: "No room for another account.",

  show: "Show",
  section: (key) =>
    ({
      session: "Current session",
      weekly: "Weekly limits",
      pace: "Pace marker",
      provenance: "Cache & credits info",
    })[key],
  refreshEvery: "Refresh every",
  refreshTitle: "Each refresh is one request to the usage API",
  refreshChoice: (seconds) => (seconds === 0 ? "Manual only" : `${seconds / 60} minutes`),
  clock: "Clock",
  clockChoice: (value) => (value === "off" ? "Hidden" : "Shown"),
  timeFormat: "Time format",
  timeFormatTitle: "Applies to the clock, the reading's time and the reset lines",
  timeFormatChoice: (value) => (value === "12" ? "12-hour" : "24-hour"),
  corners: "Corners",
  cornersTitle: "How rounded the card's corners are; the window has no frame of its own",
  cornerChoice: (px) =>
    ({ 0: "Square", 6: "Slight", 12: "Rounded", 20: "Very rounded" })[px] ?? `${px}px`,
  language: "Language",
  languageChoice: (value) =>
    ({ system: "System", en: "English", ko: "한국어" })[value] ?? value,
  update: "Update",
  updateCheck: "Check",
  updateChecking: "Checking…",
  updateCurrent: "Up to date",
  updateAvailable: (version) => `Install ${version}`,
  updateInstalling: "Installing…",
  updateFailed: "Check failed",
};

const ko: Strings = {
  lang: "ko",

  settings: "설정",
  refresh: "새로 고침",
  close: "닫기",

  meterSession: "현재 세션",
  meterWeeklyAll: "이번 주 (전체 모델)",
  meterWeeklyScoped: (model) => `이번 주 (${model})`,
  meterWeeklyScopedUnknown: "이번 주 (모델별)",
  meterOther: (kind, model) => (model ? `${kind} (${model})` : kind),
  resets: (time) => `${time} 초기화`,
  resetsOn: (date, time) => `${date} ${time} 초기화`,
  paceAhead: (used, elapsed) => `시간보다 빠르게 사용 중 (${used}% 사용, ${elapsed}% 경과)`,
  paceWithin: (used, elapsed) => `페이스 이내 (${used}% 사용, ${elapsed}% 경과)`,

  accountPickerTitle: "한도는 계정 단위입니다. 이 설치본들은 서로 다른 계정으로 로그인되어 있습니다",
  cachedSuffix: " (캐시)",

  updated: "갱신",
  cached: "캐시",
  runUsage: "/usage 실행 시 갱신",
  reasons: {
    expired: "로그인 만료",
    rateLimited: "API 요청 제한 중",
    requestFailed: "API 무응답",
    noCredentials: "로그인 없음",
  },
  extraOn: "추가 사용 켜짐",
  extraShare: (percent) => `추가 사용 ${percent}%`,
  extraOff: "추가 사용 꺼짐",
  extraOffBecause: (reason) => `추가 사용 꺼짐 (${reason})`,

  noCache: "캐시된 사용량이 없습니다. Claude Code를 한 번 실행하면 만들어집니다.",
  nothingSelected: "표시할 항목이 없습니다. 설정에서 미터를 다시 켜세요.",
  readError: (message) => `사용량을 읽지 못했습니다: ${message}`,

  loginMissing: "이 컴퓨터에 Claude Code 로그인이 없습니다.",
  loginWaitingNote: "콘솔 창에서 로그인을 완료하세요.",
  loginButton: "로그인 필요",
  loginWaitingButton: "로그인 대기 중…",
  claudeMissing: "이 컴퓨터에 Claude Code가 설치되어 있지 않습니다.",
  installButton: "설치 방법 보기",
  loginFailed: (message) => `로그인을 시작하지 못했습니다: ${message}`,
  accounts: "계정",
  accountsTitle: "사용 한도는 계정별입니다. 이 컴퓨터에서 발견된 설치본도 함께 보이며, 그것은 여기서 지울 수 없습니다.",
  accountSource: (source) => `출처: ${source}`,
  accountAdded: "여기서 추가함",
  addAccount: "계정 추가",
  addAccountWaiting: "로그인 대기 중…",
  addAccountNote: "콘솔 창에서 로그인을 완료하세요.",
  addAccountFailed: (message) => `계정을 추가하지 못했습니다: ${message}`,
  removeAccount: "제거",
  accountsFull: "계정을 더 추가할 수 없습니다.",

  show: "표시",
  section: (key) =>
    ({
      session: "현재 세션",
      weekly: "주간 한도",
      pace: "페이스 마커",
      provenance: "캐시·크레딧 정보",
    })[key],
  refreshEvery: "갱신 주기",
  refreshTitle: "갱신할 때마다 사용량 API에 요청이 한 건 나갑니다",
  refreshChoice: (seconds) => (seconds === 0 ? "수동" : `${seconds / 60}분`),
  clock: "시계",
  clockChoice: (value) => (value === "off" ? "숨김" : "표시"),
  timeFormat: "시간 표기",
  timeFormatTitle: "시계, 갱신 시각, 초기화 줄에 모두 적용됩니다",
  timeFormatChoice: (value) => (value === "12" ? "12시간" : "24시간"),
  corners: "모서리",
  cornersTitle: "카드 모서리의 곡률입니다. 창 자체에는 테두리가 없습니다",
  cornerChoice: (px) =>
    ({ 0: "각지게", 6: "살짝", 12: "둥글게", 20: "많이 둥글게" })[px] ?? `${px}px`,
  language: "언어",
  languageChoice: (value) =>
    ({ system: "시스템 설정", en: "English", ko: "한국어" })[value] ?? value,
  update: "업데이트",
  updateCheck: "확인",
  updateChecking: "확인 중…",
  updateCurrent: "최신 버전입니다",
  updateAvailable: (version) => `${version} 설치`,
  updateInstalling: "설치 중…",
  updateFailed: "확인 실패",
};

const CATALOGUES: Record<Lang, Strings> = { en, ko };

/**
 * Which language to write in.
 *
 * `system` follows what the machine is set to display, which is what the
 * setting defaults to; the explicit choices are for when that guesses wrong.
 * The answer also decides how dates and times are written, so that the card
 * reads as one language rather than two.
 */
export function resolveLang(setting: string, display = navigator.language): Lang {
  if (setting === "en" || setting === "ko") return setting;
  return display.toLowerCase().startsWith("ko") ? "ko" : "en";
}

export function strings(lang: Lang): Strings {
  return CATALOGUES[lang];
}

/**
 * A meter's name, composed here rather than sent by the backend: the wording is
 * the card's business, and the backend has no language to write it in.
 */
export function meterLabel(m: Meter, t: Strings): string {
  if (m.kind === "session") return t.meterSession;
  if (m.kind === "weekly_all") return t.meterWeeklyAll;
  if (m.kind === "weekly_scoped") {
    return m.scopeModel ? t.meterWeeklyScoped(m.scopeModel) : t.meterWeeklyScopedUnknown;
  }
  return t.meterOther(m.kind.replace(/_/g, " "), m.scopeModel);
}
