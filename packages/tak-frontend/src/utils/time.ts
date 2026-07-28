import type { GameSettings } from '@/api/game';
import { differenceInMinutes, isAfter, startOfDay } from 'date-fns';

export function timeFormat(milliseconds: number): string {
  const days = Math.floor(milliseconds / (24 * 60 * 60 * 1000));
  const hours = Math.floor((milliseconds % (24 * 60 * 60 * 1000)) / (60 * 60 * 1000));
  const minutes = Math.floor((milliseconds % (60 * 60 * 1000)) / 60000);
  const seconds = Math.floor((milliseconds % 60000) / 1000);
  const millis = milliseconds % 1000;
  const parts = [];
  if (days > 0) {
    parts.push(`${days.toString()} ${days === 1 ? 'day' : 'days'}`);
  }
  if (hours > 0) {
    parts.push(`${hours.toString()} ${hours === 1 ? 'hr' : 'hrs'}`);
  }
  if (minutes > 0) {
    parts.push(`${minutes.toString()} min`);
  }
  if (seconds > 0) {
    parts.push(`${seconds.toString()} s`);
  }
  if (millis > 0 || parts.length === 0) {
    parts.push(`${millis.toString()} ms`);
  }
  return parts.join(' ');
}

export function timeControlToString(settings: GameSettings['timeSettings']): string {
  if (settings.type === 'realtime') {
    let res = `${timeFormat(settings.contingentMs)} + ${timeFormat(settings.incrementMs)}`;
    if (settings.extra) {
      res += ` (+ ${timeFormat(settings.extra.extraMs)} @ move ${settings.extra.onMove.toString()})`;
    }
    return res;
  } else {
    return `${timeFormat(settings.contingentMs)} per move`;
  }
}

export function clockFormat(milliseconds: number): string {
  const days = Math.floor(milliseconds / (24 * 60 * 60 * 1000));
  const hours = Math.floor((milliseconds % (24 * 60 * 60 * 1000)) / (60 * 60 * 1000));
  const minutes = Math.floor((milliseconds % (60 * 60 * 1000)) / 60000);
  const seconds = Math.floor((milliseconds % 60000) / 1000);
  const millis = Math.floor(milliseconds % 1000);
  if (days > 0) {
    const paddedHours = hours.toString().padStart(2, '0');
    return `${days.toString()}d ${paddedHours}h`;
  } else if (hours > 0) {
    const paddedMinutes = minutes.toString().padStart(2, '0');
    return `${hours.toString()}h ${paddedMinutes}m`;
  } else if (minutes > 0 || seconds >= 10) {
    const paddedMinutes = minutes.toString().padStart(2, '0');
    const paddedSeconds = seconds.toString().padStart(2, '0');
    return `${paddedMinutes}:${paddedSeconds}`;
  } else {
    const paddedSeconds = seconds.toString().padStart(2, '0');
    const paddedMilliseconds = millis.toString().padStart(3, '0');
    return `${paddedSeconds}.${paddedMilliseconds}`;
  }
}

export function areTimestampsDifferentMinutes(t1: number, t2: number): boolean {
  const date1 = new Date(t1);
  const date2 = new Date(t2);
  return differenceInMinutes(date1, date2) !== 0 || date1.getUTCMinutes() !== date2.getUTCMinutes();
}

export function isToday(t1: number): boolean {
  const startOfToday = startOfDay(new Date());
  const date1 = new Date(t1);
  return isAfter(date1, startOfToday);
}
