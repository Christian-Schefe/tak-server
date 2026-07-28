import { immerable } from 'immer';
import {
  playerOpponent,
  type TakAction,
  type TakAsyncTimeControl,
  type TakGameResult,
  type TakGameSettings,
  type TakPlayer,
  type TakRealtimeTimeControl,
} from '.';
import { TakBaseGame } from './base';

export interface TakClock {
  remainingTimeMs: Record<TakPlayer, number>;
  lastUpdateTimestamp: number;
  isTicking: boolean;
}

export type TakClockUpdatePolicy =
  | {
      type: 'realtime';
      timeControl: TakRealtimeTimeControl;
      hasGainedExtraTime: Record<TakPlayer, boolean>;
    }
  | {
      type: 'async';
      timeControl: TakAsyncTimeControl;
    };

export class TakGame {
  [immerable] = true;

  base: TakBaseGame;
  clock: TakClock;
  clockUpdatePolicy: TakClockUpdatePolicy;

  constructor(settings: TakGameSettings) {
    this.base = new TakBaseGame(settings.base);
    this.clock = {
      remainingTimeMs: {
        white: settings.timeControl.contingentMs,
        black: settings.timeControl.contingentMs,
      },
      lastUpdateTimestamp: Date.now(),
      isTicking: false,
    };
    this.clockUpdatePolicy =
      settings.timeControl.type === 'realtime'
        ? {
            type: settings.timeControl.type,
            timeControl: settings.timeControl,
            hasGainedExtraTime: {
              white: false,
              black: false,
            },
          }
        : {
            type: settings.timeControl.type,
            timeControl: settings.timeControl,
          };
  }

  setGameOver(gameResult: TakGameResult, now: number) {
    this.stopClock(this.base.currentPlayer, now);
    this.base.gameResult = gameResult;
  }

  private stopClock(player: TakPlayer, now: number) {
    this.maybeApplyElapsed(player, now);
    this.clock.isTicking = false;
  }

  private maybeApplyElapsed(player: TakPlayer, now: number) {
    if (this.clock.isTicking) {
      const elapsed = now - this.clock.lastUpdateTimestamp;
      this.clock.remainingTimeMs[player] = Math.max(
        this.clock.remainingTimeMs[player] - elapsed,
        0,
      );
    }
    this.clock.lastUpdateTimestamp = now;
  }

  getTimeRemaining(player: TakPlayer, now: number): number {
    const baseRemaining = this.clock.remainingTimeMs[player];
    if (this.base.currentPlayer !== player || !this.clock.isTicking) {
      return baseRemaining;
    }
    const elapsed = now - this.clock.lastUpdateTimestamp;
    return Math.max(baseRemaining - elapsed, 0);
  }

  private startOrUpdateClock(player: TakPlayer, now: number) {
    this.maybeApplyElapsed(player, now);
    switch (this.clockUpdatePolicy.type) {
      case 'realtime': {
        this.clock.remainingTimeMs[player] += this.clockUpdatePolicy.timeControl.incrementMs;
        if (
          this.clockUpdatePolicy.timeControl.extra !== null &&
          !this.clockUpdatePolicy.hasGainedExtraTime[player]
        ) {
          const moveIndex = (this.base.actionHistory.length + 1) / 2;

          if (moveIndex === this.clockUpdatePolicy.timeControl.extra.onMove) {
            this.clock.remainingTimeMs[player] += this.clockUpdatePolicy.timeControl.extra.extraMs;
            this.clockUpdatePolicy.hasGainedExtraTime[player] = true;
          }
        }
        break;
      }
      case 'async': {
        this.clock.remainingTimeMs.white = this.clockUpdatePolicy.timeControl.contingentMs;
        this.clock.remainingTimeMs.black = this.clockUpdatePolicy.timeControl.contingentMs;
        break;
      }
    }
    this.clock.isTicking = true;
  }

  setTimeRemaining(remainingMs: Record<TakPlayer, number>, now: number) {
    this.clock.remainingTimeMs = { ...remainingMs };
    this.clock.lastUpdateTimestamp = now;
  }

  private checkTimeout(now: number): boolean {
    const player = this.base.currentPlayer;
    const timeRemaining = this.getTimeRemaining(player, now);
    if (timeRemaining <= 0) {
      const gameResult: TakGameResult = {
        type: 'win',
        winner: playerOpponent(player),
        reason: 'default',
      };
      this.setGameOver(gameResult, now);
    }
    return false;
  }

  doAction(action: TakAction, now: number): boolean {
    const gameResult = this.base.gameResult;
    if (gameResult !== null) {
      return false;
    }
    if (this.checkTimeout(now)) {
      return false;
    }
    const player = this.base.currentPlayer;
    if (!this.base.canDoAction(action)) {
      return false;
    }
    this.base.doAction(action);
    if (this.base.gameResult !== null) {
      this.stopClock(player, now);
    } else {
      this.startOrUpdateClock(player, now);
    }
    return true;
  }

  undoAction(now: number): boolean {
    if (this.base.gameResult !== null) {
      return false;
    }
    if (this.checkTimeout(now)) {
      return false;
    }
    const player = this.base.currentPlayer;
    if (!this.base.canUndoAction()) {
      return false;
    }
    this.base.undoAction();
    this.startOrUpdateClock(player, now);
    return true;
  }
}
